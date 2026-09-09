use std::io::{BufRead, Write};

use super::{EventKind, EventMeta, InputEvent, InputList, OutputList, RawElement, Spacing};
use crate::errors::{Error, Result};
use crate::types::OrderIndex;

use quick_xml::escape::{minimal_escape, unescape};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event as XmlEvent};
use quick_xml::{Reader, Writer};

// Type wrapper to avoid leaking XmlEvent from this module
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawXmlEvent(XmlEvent<'static>);

// Type wrapper to avoid leaking BytesStart from this module
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTag(BytesStart<'static>);

impl EventKind {
    pub fn is_eof(&self) -> bool {
        matches!(self, EventKind::Other ( event ) if matches!(event.0, XmlEvent::Eof))
    }

    pub fn tag_name(&self) -> Option<String> {
        match self {
            EventKind::Start(element) | EventKind::Empty(element) => {
                Some(element.name().to_owned())
            }
            EventKind::RawStart(bs) | EventKind::RawEmpty(bs) => {
                Some(bs.0.name().into_inner().to_string())
            }
            EventKind::End(name) => Some(name.clone()),
            _ => None,
        }
    }
}

impl TryFrom<XmlEvent<'_>> for EventKind {
    type Error = Error;
    fn try_from(event: XmlEvent) -> Result<Self> {
        let res = match event {
            XmlEvent::Empty(bs) => EventKind::Empty(bs.try_into()?),
            XmlEvent::Start(bs) => EventKind::Start(bs.try_into()?),
            XmlEvent::End(e) => {
                let name = e.name().into_inner().to_string();
                EventKind::End(name)
            }
            XmlEvent::Text(t) => {
                let content = t.into_inner().to_string();
                EventKind::Text(content)
            }
            XmlEvent::CData(c) => {
                let content = c.into_inner().to_string();
                EventKind::CData(content)
            }
            XmlEvent::Comment(c) => {
                let content = c.into_inner().to_string();
                EventKind::Comment(content)
            }
            other => EventKind::Other(RawXmlEvent(other.into_owned())),
        };
        Ok(res)
    }
}

impl EventKind {
    pub fn raw_from(xe: XmlEvent<'_>) -> Result<Self> {
        match xe {
            XmlEvent::Empty(bs) => Ok(EventKind::RawEmpty(RawTag(normalize_tag(&bs)?))),
            XmlEvent::Start(bs) => Ok(EventKind::RawStart(RawTag(normalize_tag(&bs)?))),
            other => other.try_into(),
        }
    }
}

impl<'a> From<EventKind> for XmlEvent<'a> {
    fn from(svg_ev: EventKind) -> XmlEvent<'a> {
        match svg_ev {
            EventKind::Empty(e) => XmlEvent::Empty(e.into()),
            EventKind::Start(e) => XmlEvent::Start(e.into()),
            EventKind::Comment(content) => XmlEvent::Comment(BytesText::from_escaped(content)),
            EventKind::Text(content) => {
                XmlEvent::Text(BytesText::from_escaped(minimal_escape(content)))
            }
            EventKind::CData(content) => XmlEvent::CData(BytesCData::new(content)),
            EventKind::End(name) => XmlEvent::End(BytesEnd::new(name)),
            // Spacing markers should already have been consumed by `OutputList::write_to`.
            // If one leaks through, serialize as empty text node.
            EventKind::Spacing(_) => XmlEvent::Text(BytesText::new("")),
            EventKind::Other(event) => event.0,
            EventKind::RawEmpty(bs) => XmlEvent::Empty(bs.0),
            EventKind::RawStart(bs) => XmlEvent::Start(bs.0),
        }
    }
}

impl TryFrom<BytesStart<'_>> for RawElement {
    type Error = Error;

    /// Build a `RawElement` from a `BytesStart` value. Failures here are are low-level
    /// XML type errors (e.g. bad attribute names, non-UTF8) rather than anything
    /// semantic about svgdx / svg formats.
    fn try_from(e: BytesStart) -> Result<Self> {
        let name = e.name().into_inner().to_string();
        // TODO: in a non-strict mode, consider .filter_map(|a| { a.ok().and_then(|aa| { ...
        let attrs: Result<Vec<(String, String)>> = e
            .attributes()
            .map(move |a| {
                let aa = a.map_err(|e| Error::Xml(Box::new(e)))?;
                let key = aa.key.into_inner().to_string();
                let raw_value = aa.value.as_ref();
                let value = unescape(raw_value)
                    .map_err(|e| Error::Xml(Box::new(e)))?
                    .into_owned();
                Ok((key, value))
            })
            .collect();
        Ok(Self(name, attrs?))
    }
}

impl From<RawElement> for BytesStart<'static> {
    fn from(e: RawElement) -> Self {
        let mut bs = BytesStart::new(e.0);
        for (k, v) in e.1 {
            bs.push_attribute(Attribute::from((k.as_str(), v.as_str())));
        }
        bs
    }
}

impl InputList {
    pub fn from_reader(reader: &mut dyn BufRead) -> Result<Self> {
        Self::from_reader_impl(reader, false)
    }

    pub fn from_reformat_reader(reader: &mut dyn BufRead) -> Result<Self> {
        Self::from_reader_impl(reader, true)
    }

    fn from_reader_impl(reader: &mut dyn BufRead, raw: bool) -> Result<Self> {
        let mut reader = Reader::from_reader(reader);

        let mut events = Vec::new();
        let mut buf = Vec::new();

        // Stack of indices of open tags
        let mut event_idx_stack = Vec::new();

        let mut src_line = 1;
        let mut indent = 0;
        let mut index = 0;
        let mut order = OrderIndex::new(1);
        loop {
            let ev = reader.read_event_into(&mut buf);
            let event_lines = if let Ok(ok_ev) = ev.clone() {
                ok_ev.as_bytes().iter().filter(|&c| *c == b'\n').count()
            } else {
                0
            };
            let ev =
                ev.map_err(|e| Error::Document(format!("XML error near line {src_line}: {e:?}")))?;

            if matches!(ev, XmlEvent::Eof) {
                break;
            }

            let mut meta = EventMeta {
                index,
                order: order.clone(),
                line: src_line,
                indent,
                alt_idx: None,
                depth: event_idx_stack.len(),
            };

            let e: EventKind = if raw {
                EventKind::raw_from(ev)?
            } else {
                ev.clone().try_into()?
            };
            if e.is_eof() {
                break;
            }

            match e {
                EventKind::Text(content) => {
                    let mut t_str = content.clone();
                    if let Some((_, rest)) = t_str.rsplit_once('\n') {
                        t_str = rest.to_string();
                    }
                    indent = t_str.len() - t_str.trim_end_matches(' ').len();
                    meta.indent = indent;
                    events.push(InputEvent {
                        event: EventKind::Text(content),
                        meta,
                    });
                    order.step();
                }
                EventKind::Start(_) | EventKind::RawStart(_) => {
                    events.push(InputEvent { event: e, meta });
                    event_idx_stack.push(index);
                    order.down();
                }
                EventKind::End(name) => {
                    let start_idx = event_idx_stack.pop();
                    if let Some(start_idx) = start_idx {
                        events[start_idx].meta.alt_idx = Some(index);
                    }
                    order.up();
                    meta.alt_idx = start_idx;
                    events.push(InputEvent {
                        event: EventKind::End(name),
                        meta,
                    });
                    order.step();
                }
                e => {
                    events.push(InputEvent { event: e, meta });
                    order.step();
                }
            }

            src_line += event_lines;
            index += 1;
            buf.clear();
        }

        // println!("event list:");
        // for ev in &events {
        //     println!(" {}: {:?}", ev.order.to_string(), ev.event);
        // }
        // println!("");

        if let Some(start_idx) = event_idx_stack.last().copied() {
            let element_name = events[start_idx]
                .event
                .tag_name()
                .unwrap_or_else(|| "unknown".to_owned());
            return Err(Error::Document(format!(
                "XML error near line {src_line}: unclosed element <{element_name}>"
            )));
        }

        Ok(Self { events })
    }
}

/// re-generate an owned XML tag with normalized inter-attribute spacing
///
/// ```xml
/// <abc
///    pqr="xyz"   x="1">
/// ```
/// ->
/// ```xml
/// <abc pqr="xyz" x="1">
/// ```
fn normalize_tag(start: &BytesStart<'_>) -> Result<BytesStart<'static>> {
    let mut norm = BytesStart::new(start.name().into_inner().to_string());
    for attr in start.attributes().with_checks(false) {
        norm.push_attribute(attr.map_err(|e| Error::Xml(Box::new(e)))?);
    }
    Ok(norm)
}

impl OutputList {
    pub fn write_to(&self, writer: &mut dyn Write) -> Result<()> {
        let mut writer = Writer::new(writer);
        let mut pending_spacing = Spacing::default();
        let mut text_buf = String::new();
        let mut depth = 0usize;
        let mut parent_stack: Vec<String> = Vec::new();

        for event_pos in &self.events {
            match &event_pos.event {
                EventKind::Spacing(spacing) => {
                    pending_spacing.merge(*spacing);
                }
                EventKind::Text(content) => {
                    text_buf.push_str(content);
                }
                event => {
                    Self::flush_text_buf(&mut writer, &mut text_buf)?;
                    Self::write_spacing(
                        &mut writer,
                        pending_spacing.take(),
                        depth,
                        event,
                        parent_stack.last().map(String::as_str),
                    )?;

                    writer.write_event(event.clone())?;
                    match event {
                        EventKind::Start(_) | EventKind::RawStart(_) => {
                            depth += 1;
                            parent_stack.push(event.tag_name().unwrap_or_default());
                        }
                        EventKind::End(_) => {
                            depth = depth.saturating_sub(1);
                            parent_stack.pop();
                        }
                        _ => {}
                    }
                }
            }
        }

        Self::flush_text_buf(&mut writer, &mut text_buf)?;
        Self::write_end_spacing(&mut writer, pending_spacing.take())?;
        Ok(())
    }

    fn flush_text_buf(writer: &mut Writer<&mut dyn Write>, text_buf: &mut String) -> Result<()> {
        if !text_buf.is_empty() {
            let text_event = EventKind::Text(std::mem::take(text_buf));
            writer.write_event(text_event)?;
        }
        Ok(())
    }

    /// at EOF, write max of one newline
    fn write_end_spacing(writer: &mut Writer<&mut dyn Write>, spacing: Spacing) -> Result<()> {
        if spacing != Spacing::Inline {
            writer.write_event(EventKind::Text("\n".into()))?;
        }
        Ok(())
    }

    // Insert canonical structural whitespace before the next XML event.
    // Text under `tspan` is content-bearing, so formatter whitespace must not be injected there.
    fn write_spacing(
        writer: &mut Writer<&mut dyn Write>,
        spacing: Spacing,
        depth: usize,
        event: &EventKind,
        parent_name: Option<&str>,
    ) -> Result<()> {
        if matches!(parent_name, Some("tspan")) || matches!(spacing, Spacing::Inline) {
            return Ok(());
        }

        let indent_depth = match event {
            EventKind::End(_) => depth.saturating_sub(1),
            _ => depth,
        };
        let newline_count = match spacing {
            Spacing::Inline => 0,
            Spacing::LineBreak => 1,
            Spacing::BlankLine => 2,
        };

        let mut content = "\n".repeat(newline_count);
        content.push_str(&"  ".repeat(indent_depth));
        writer.write_event(EventKind::Text(content))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_eventlist_minimal() {
        let input = r#"<svg></svg>"#;
        let mut buf_input = Cursor::new(input);
        let el = InputList::from_reader(&mut buf_input).unwrap();
        assert_eq!(el.events.len(), 2);
        assert_eq!(el.events[0].meta.line, 1);
        assert_eq!(
            el.events[0].event,
            EventKind::Start(RawElement("svg".into(), vec![]))
        );
        assert_eq!(el.events[1].meta.line, 1);
        assert_eq!(el.events[1].event, EventKind::End("svg".into()));
    }

    #[test]
    fn test_eventlist_indent() {
        let input = r#"<svg>
        </svg>"#;
        let mut buf_input = Cursor::new(input);
        let el = InputList::from_reader(&mut buf_input).unwrap();
        assert_eq!(el.events.len(), 3);
        assert_eq!(el.events[0].meta.line, 1);
        assert_eq!(el.events[0].meta.indent, 0);
        assert_eq!(
            el.events[0].event,
            EventKind::Start(RawElement("svg".into(), vec![]))
        );
        // Multi-line events (e.g. text in this instance) store starting line number
        assert_eq!(el.events[1].meta.line, 1);
        assert_eq!(el.events[1].event, EventKind::Text("\n        ".into()));
        assert_eq!(el.events[2].meta.line, 2);
        assert_eq!(el.events[2].meta.indent, 8);
        assert_eq!(el.events[2].event, EventKind::End("svg".into()));
    }

    #[test]
    fn test_outputlist_write_to() {
        let input = r#"<svg><rect width="100" height="100"/></svg>"#;
        let mut buf_input = Cursor::new(input);
        let input_list = InputList::from_reader(&mut buf_input).unwrap();
        let output_list: OutputList = input_list.into();

        let mut cursor = Cursor::new(Vec::new());
        output_list.write_to(&mut cursor).unwrap();

        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_multiline_attribute_value() {
        // Check parser doesn't squish whitespace in multiline attributes,
        // which XML parsers could reasonably do.
        let input = r#"<svg><path d="
    M 10 10
    L 20 20
"/></svg>"#;
        let mut buf_input = Cursor::new(input);
        let input_list = InputList::from_reader(&mut buf_input).unwrap();

        assert_eq!(input_list.events.len(), 3);
        assert_eq!(
            input_list.events[1].event,
            EventKind::Empty(RawElement(
                "path".into(),
                vec![("d".into(), "\n    M 10 10\n    L 20 20\n".into())]
            ))
        );
    }

    #[test]
    fn test_multiline_text_content() {
        // Check parser doesn't squish whitespace in multiline text content
        let input = "<svg><text>\n    first line\n    second line\n</text></svg>";
        let mut buf_input = Cursor::new(input);
        let input_list = InputList::from_reader(&mut buf_input).unwrap();
        let output_list: OutputList = input_list.into();

        let mut cursor = Cursor::new(Vec::new());
        output_list.write_to(&mut cursor).unwrap();

        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_reformat_reader_preserves_multiline_attrs() {
        let input = r#"<svg><rect text="
    line one
line two
"/></svg>"#;

        let mut buf_input = Cursor::new(input);
        let output_list = InputList::from_reformat_reader(&mut buf_input)
            .unwrap()
            .reformat();

        let mut cursor = Cursor::new(Vec::new());
        output_list.write_to(&mut cursor).unwrap();

        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_reformat_reader_preserves_comments() {
        let input = r#"<svg><rect wh="2"/>
<!--
This is a comment
  Split over multiple lines
  -->
</svg>"#;

        let expected = r#"<!--
This is a comment
  Split over multiple lines
  -->"#;

        let mut buf_input = Cursor::new(input);
        let output_list = InputList::from_reformat_reader(&mut buf_input)
            .unwrap()
            .reformat();

        let mut cursor = Cursor::new(Vec::new());
        output_list.write_to(&mut cursor).unwrap();

        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert!(result.contains(expected));
    }
}
