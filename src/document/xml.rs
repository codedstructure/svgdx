use std::io::{BufRead, Write};

use super::{EventKind, EventMeta, InputEvent, InputList, OutputList, RawElement};
use crate::errors::{Error, Result};
use crate::types::OrderIndex;

use quick_xml::escape::partial_escape;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event as XmlEvent};
use quick_xml::{Reader, Writer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawXmlEvent(XmlEvent<'static>);

impl EventKind {
    pub fn is_eof(&self) -> bool {
        matches!(self, EventKind::Other ( event ) if matches!(event.0, XmlEvent::Eof))
    }
}

impl TryFrom<XmlEvent<'_>> for EventKind {
    type Error = Error;
    fn try_from(event: XmlEvent) -> Result<Self> {
        let res = match event {
            XmlEvent::Empty(bs) => EventKind::Empty(bs.try_into()?),
            XmlEvent::Start(bs) => EventKind::Start(bs.try_into()?),
            XmlEvent::End(e) => {
                let name = String::from_utf8(e.name().into_inner().to_vec()).expect("utf8");
                EventKind::End(name)
            }
            XmlEvent::Text(t) => {
                let content = String::from_utf8(t.into_inner().to_vec()).expect("utf8");
                EventKind::Text(content)
            }
            XmlEvent::CData(c) => {
                let content = String::from_utf8(c.into_inner().to_vec()).expect("utf8");
                EventKind::CData(content)
            }
            XmlEvent::Comment(c) => {
                let content = String::from_utf8(c.into_inner().to_vec()).expect("utf8");
                EventKind::Comment(content)
            }
            other => EventKind::Other(RawXmlEvent(other.into_owned())),
        };
        Ok(res)
    }
}

impl<'a> From<EventKind> for XmlEvent<'a> {
    fn from(svg_ev: EventKind) -> XmlEvent<'a> {
        match svg_ev {
            EventKind::Empty(e) => XmlEvent::Empty(e.into()),
            EventKind::Start(e) => XmlEvent::Start(e.into()),
            EventKind::Comment(content) => XmlEvent::Comment(BytesText::from_escaped(content)),
            EventKind::Text(content) => {
                XmlEvent::Text(BytesText::from_escaped(partial_escape(content)))
            }
            EventKind::CData(content) => XmlEvent::CData(BytesCData::new(content)),
            EventKind::End(name) => XmlEvent::End(BytesEnd::new(name)),
            EventKind::Other(event) => event.0,
        }
    }
}

impl TryFrom<BytesStart<'_>> for RawElement {
    type Error = Error;

    /// Build a `RawElement` from a `BytesStart` value. Failures here are are low-level
    /// XML type errors (e.g. bad attribute names, non-UTF8) rather than anything
    /// semantic about svgdx / svg formats.
    fn try_from(e: BytesStart) -> Result<Self> {
        let name = String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");
        // TODO: in a non-strict mode, consider .filter_map(|a| { a.ok().and_then(|aa| { ...
        let attrs: Result<Vec<(String, String)>> = e
            .attributes()
            .map(move |a| {
                let aa = a.map_err(Error::from_err)?;
                let key = String::from_utf8(aa.key.into_inner().to_vec())?;
                let value = aa.unescape_value().map_err(Error::from_err)?.into_owned();
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
            bs.push_attribute(Attribute::from((k.as_bytes(), v.as_bytes())));
        }
        bs
    }
}

impl InputList {
    pub fn from_reader(reader: &mut dyn BufRead) -> Result<Self> {
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
                ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count()
            } else {
                0
            };
            let ev =
                ev.map_err(|e| Error::Document(format!("XML error near line {src_line}: {e:?}")))?;
            let mut meta = EventMeta {
                index,
                order: order.clone(),
                line: src_line,
                indent,
                alt_idx: None,
            };

            let e: EventKind = ev.clone().try_into()?;
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
                EventKind::Start(el) => {
                    events.push(InputEvent {
                        event: EventKind::Start(el),
                        meta,
                    });
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

        Ok(Self { events })
    }
}

impl OutputList {
    pub fn write_to(&self, writer: &mut dyn Write) -> Result<()> {
        let mut writer = Writer::new(writer);

        // Separate buffer for coalescing text events
        let mut text_buf = String::new();

        for event_pos in &self.events {
            let event = event_pos.event.clone();
            if let EventKind::Text(ref content) = event {
                text_buf.push_str(content);
                continue;
            } else if !text_buf.is_empty() {
                let content = Self::blank_line_remover(&text_buf);
                let text_event = EventKind::Text(content);
                text_buf.clear();
                writer.write_event(text_event).map_err(Error::from_err)?;
            }
            writer.write_event(event).map_err(Error::from_err)?;
        }
        // re-add any trailing text
        if !text_buf.is_empty() {
            let content = Self::blank_line_remover(&text_buf);
            let text_event = EventKind::Text(content);
            writer.write_event(text_event).map_err(Error::from_err)?;
        }
        Ok(())
    }

    fn blank_line_remover(s: &str) -> String {
        // trim trailing whitespace.
        // just using `trim_end()` on Text events won't work
        // as Text event may be followed by a Start/Empty event.
        // blank lines *within* Text can be trimmed.
        let mut content = String::new();
        let mut s = s;
        while !s.is_empty() {
            if let Some(idx) = s.find('\n') {
                let (line, remain) = s.split_at(idx);
                s = &remain[1..];
                content.push_str(line.trim_end());
                content.push('\n');
            } else {
                content.push_str(s);
                break;
            }
        }
        content
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
}
