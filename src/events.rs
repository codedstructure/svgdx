use crate::elements::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::types::OrderIndex;

use std::io::{BufRead, BufReader, Cursor, Write};
use std::str::FromStr;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEvent {
    event: Event<'static>,
    pub index: usize,
    line: usize,
    indent: usize,
    pub alt_idx: Option<usize>,
}

impl InputEvent {
    pub fn text_string(&self) -> Option<String> {
        match &self.event {
            Event::Text(t) => Some(String::from_utf8(t.to_vec()).expect("utf8")),
            _ => None,
        }
    }

    pub fn cdata_string(&self) -> Option<String> {
        match &self.event {
            Event::CData(c) => Some(String::from_utf8(c.to_vec()).expect("utf8")),
            _ => None,
        }
    }
}

impl From<Event<'_>> for InputEvent {
    fn from(value: Event) -> Self {
        Self {
            event: value.into_owned(),
            index: 0,
            line: 0,
            indent: 0,
            alt_idx: None,
        }
    }
}

impl From<OutputEvent> for InputEvent {
    fn from(value: OutputEvent) -> Self {
        InputEvent::from(Event::from(value))
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct InputList {
    pub events: Vec<InputEvent>,
}

impl From<&[InputEvent]> for InputList {
    fn from(value: &[InputEvent]) -> Self {
        Self {
            events: value
                .iter()
                .map(|v| InputEvent {
                    event: v.event.clone(),
                    index: v.index,
                    line: v.line,
                    indent: v.indent,
                    alt_idx: v.alt_idx,
                })
                .collect(),
        }
    }
}

impl IntoIterator for InputList {
    type Item = InputEvent;
    type IntoIter = std::vec::IntoIter<InputEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl FromStr for InputList {
    type Err = SvgdxError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_reader(&mut BufReader::new(Cursor::new(s.as_bytes())))
    }
}

impl InputList {
    pub fn new() -> Self {
        Self { events: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InputEvent> + '_ {
        self.events.iter()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn push(&mut self, ev: impl Into<InputEvent>) {
        let ev = ev.into();
        self.events.push(ev.clone());
    }

    pub fn extend(&mut self, other: &InputList) {
        for ev in other.iter().cloned() {
            self.events.push(ev);
        }
    }

    pub fn from_reader(reader: &mut dyn BufRead) -> Result<Self> {
        let mut reader = Reader::from_reader(reader);

        let mut events = Vec::new();
        let mut buf = Vec::new();

        // Stack of indices of open tags
        let mut event_idx_stack = Vec::new();

        let mut src_line = 1;
        let mut indent = 0;
        let mut index = 0;
        loop {
            let ev = reader.read_event_into(&mut buf);
            let event_lines = if let Ok(ok_ev) = ev.clone() {
                ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count()
            } else {
                0
            };
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Text(t)) => {
                    let mut t_str = String::from_utf8(t.to_vec())?;
                    if let Some((_, rest)) = t_str.rsplit_once('\n') {
                        t_str = rest.to_string();
                    }
                    indent = t_str.len() - t_str.trim_end_matches(' ').len();

                    events.push(InputEvent {
                        event: ev.expect("match").into_owned(),
                        index,
                        line: src_line,
                        indent,
                        alt_idx: None,
                    });
                }
                Ok(Event::Start(_)) => {
                    events.push(InputEvent {
                        event: ev.expect("match").into_owned(),
                        index,
                        line: src_line,
                        indent,
                        alt_idx: None,
                    });
                    event_idx_stack.push(index);
                }
                Ok(Event::End(_)) => {
                    let start_idx = event_idx_stack.pop();
                    if let Some(start_idx) = start_idx {
                        events[start_idx].alt_idx = Some(index);
                    }
                    events.push(InputEvent {
                        event: ev.expect("match").into_owned(),
                        index,
                        line: src_line,
                        indent,
                        alt_idx: start_idx,
                    });
                }
                Ok(e) => events.push(InputEvent {
                    event: e.clone().into_owned(),
                    index,
                    line: src_line,
                    indent,
                    alt_idx: None,
                }),
                Err(e) => {
                    return Err(SvgdxError::ParseError(format!(
                        "XML error near line {src_line}: {e:?}"
                    )))
                }
            }

            src_line += event_lines;
            index += 1;
            buf.clear();
        }

        Ok(Self { events })
    }

    pub fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            events: self.events[start..end].to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Tag {
    /// Represents a Start..End block and all events in between
    Compound(SvgElement, Option<String>),
    /// Represents a single Empty element
    Leaf(SvgElement, Option<String>),
    Comment(String, Option<String>),
    Text(String),
    CData(String),
}

impl Tag {
    fn set_text(&mut self, text: String) {
        match self {
            Tag::Compound(_, tail) => *tail = Some(text),
            Tag::Leaf(_, tail) => *tail = Some(text),
            Tag::Comment(_, tail) => *tail = Some(text),
            _ => {}
        }
    }

    pub fn get_element(&self) -> Option<SvgElement> {
        match self {
            Tag::Compound(el, _) => Some(el.clone()),
            Tag::Leaf(el, _) => Some(el.clone()),
            _ => None,
        }
    }
}

// Provide a list of tags which can be processed in-order.
pub fn tagify_events(events: InputList) -> Result<Vec<Tag>> {
    let mut tags = Vec::new();
    let mut ev_idx = 0;

    // we use indexed iteration as we need to skip ahead in some cases
    while ev_idx < events.len() {
        let input_ev = &events.events[ev_idx];
        ev_idx += 1;
        let ev = &input_ev.event;
        match ev {
            Event::Start(_) => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    SvgdxError::DocumentError(format!(
                        "could not extract element at line {}",
                        input_ev.line
                    ))
                })?;
                if let Some(alt_idx) = input_ev.alt_idx {
                    event_element.set_event_range((input_ev.index, alt_idx));
                    // Scan ahead to the end of this element, matching alt_idx.
                    // Note when called recursively on a subset of events, alt_idx
                    // won't be the same as next_idx, so we need to scan rather than
                    // just setting ev_idx = alt_idx + 1.
                    for next_idx in ev_idx..events.len() {
                        if events.events[next_idx].index == alt_idx {
                            ev_idx = next_idx + 1; // skip the End event itself
                            break;
                        }
                    }
                } // TODO: else warning message
                tags.push(Tag::Compound(event_element, None));
            }
            Event::Empty(_) => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    SvgdxError::DocumentError(format!(
                        "could not extract element at line {}",
                        input_ev.line
                    ))
                })?;
                event_element.set_event_range((input_ev.index, input_ev.index));
                tags.push(Tag::Leaf(event_element, None));
            }
            Event::Comment(c) => {
                let text = String::from_utf8(c.to_vec())?;
                tags.push(Tag::Comment(text, None));
            }
            Event::Text(t) => {
                let text = String::from_utf8(t.to_vec())?;
                if let Some(t) = tags.last_mut() {
                    t.set_text(text)
                } else {
                    tags.push(Tag::Text(text));
                }
            }
            Event::CData(c) => {
                let text = String::from_utf8(c.to_vec())?;
                if let Some(t) = tags.last_mut() {
                    t.set_text(text)
                } else {
                    tags.push(Tag::CData(text));
                }
            }
            _ => {
                // This would include Event::End, as well as PI, DocType, etc.
                // Specifically End shouldn't be seen due to alt_idx scan-ahead.
            }
        }
    }
    Ok(tags)
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputEvent {
    Comment(String),
    Text(String),
    CData(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
    Other(Event<'static>),
}

impl From<InputEvent> for OutputEvent {
    fn from(value: InputEvent) -> Self {
        match value.event {
            Event::Empty(ref e) => {
                if let Ok(el) = SvgElement::try_from(e) {
                    OutputEvent::Empty(el)
                } else {
                    OutputEvent::Other(value.event)
                }
            }
            Event::Start(ref e) => {
                if let Ok(el) = SvgElement::try_from(e) {
                    OutputEvent::Start(el)
                } else {
                    OutputEvent::Other(value.event)
                }
            }
            Event::End(e) => {
                let elem_name: String =
                    String::from_utf8(e.name().into_inner().to_vec()).expect("utf8");
                OutputEvent::End(elem_name)
            }
            Event::Text(t) => {
                OutputEvent::Text(String::from_utf8(t.into_inner().to_vec()).expect("utf8"))
            }
            Event::CData(c) => {
                OutputEvent::CData(String::from_utf8(c.into_inner().to_vec()).expect("utf8"))
            }
            Event::Comment(c) => {
                OutputEvent::Comment(String::from_utf8(c.into_inner().to_vec()).expect("utf8"))
            }
            _ => OutputEvent::Other(value.event),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct OutputList {
    events: Vec<OutputEvent>,
}

impl From<&[OutputEvent]> for OutputList {
    fn from(value: &[OutputEvent]) -> Self {
        Self {
            events: value.to_vec(),
        }
    }
}

impl From<Vec<OutputEvent>> for OutputList {
    fn from(value: Vec<OutputEvent>) -> Self {
        Self { events: value }
    }
}

impl From<InputList> for OutputList {
    fn from(value: InputList) -> Self {
        Self {
            events: value.events.into_iter().map(|ev| ev.into()).collect(),
        }
    }
}

impl OutputList {
    pub fn new() -> Self {
        Self { events: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &OutputEvent> + '_ {
        self.events.iter()
    }

    pub fn push(&mut self, ev: impl Into<OutputEvent>) {
        let ev = ev.into();
        self.events.push(ev.clone());
    }

    pub fn extend(&mut self, other: &OutputList) {
        self.events.extend(other.events.clone());
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

    pub fn write_to(&self, writer: &mut dyn Write) -> Result<()> {
        let mut writer = Writer::new(writer);

        // Separate buffer for coalescing text events
        let mut text_buf = String::new();

        for event_pos in &self.events {
            let event = event_pos.clone();
            if let OutputEvent::Text(ref content) = event {
                text_buf.push_str(content);
                continue;
            } else if !text_buf.is_empty() {
                let content = Self::blank_line_remover(&text_buf);
                let text_event = Event::Text(BytesText::new(&content).into_owned());
                text_buf.clear();
                writer
                    .write_event(text_event)
                    .map_err(SvgdxError::from_err)?;
            }
            writer.write_event(event).map_err(SvgdxError::from_err)?;
        }
        // re-add any trailing text
        if !text_buf.is_empty() {
            let content = Self::blank_line_remover(&text_buf);
            let text_event = Event::Text(BytesText::new(&content).into_owned());
            writer
                .write_event(text_event)
                .map_err(SvgdxError::from_err)?;
        }
        Ok(())
    }

    /// Split an `OutputList` into (up to) 3 parts: before, pivot, after.
    pub fn partition(&self, name: &str) -> (Self, Option<OutputEvent>, Self) {
        let mut before = vec![];
        let mut pivot = None;
        let mut after = vec![];
        for output_ev in self.events.clone() {
            if pivot.is_some() {
                after.push(output_ev);
            } else {
                match &output_ev {
                    OutputEvent::Start(e) | OutputEvent::Empty(e) => {
                        if e.name() == name {
                            pivot = Some(output_ev);
                        } else {
                            before.push(output_ev);
                        }
                    }
                    _ => before.push(output_ev),
                }
            }
        }

        (Self { events: before }, pivot, Self { events: after })
    }
}

impl IntoIterator for OutputList {
    type Item = OutputEvent;
    type IntoIter = std::vec::IntoIter<OutputEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl<'a> From<OutputEvent> for Event<'a> {
    fn from(svg_ev: OutputEvent) -> Event<'a> {
        match svg_ev {
            OutputEvent::Empty(e) => Event::Empty(e.into_bytesstart()),
            OutputEvent::Start(e) => Event::Start(e.into_bytesstart()),
            OutputEvent::Comment(t) => Event::Comment(BytesText::from_escaped(t)),
            OutputEvent::Text(t) => Event::Text(BytesText::from_escaped(t)),
            OutputEvent::CData(t) => Event::CData(BytesCData::new(t)),
            OutputEvent::End(name) => Event::End(BytesEnd::new(name)),
            OutputEvent::Other(e) => e,
        }
    }
}

impl SvgElement {
    /// Convert an `SvgElement` into a `BytesStart` value.
    ///
    /// Implemented as a method rather than a `From` impl to keep private
    fn into_bytesstart(self) -> BytesStart<'static> {
        let mut bs = BytesStart::new(self.name().to_owned());
        for (k, v) in &self.attrs {
            bs.push_attribute(Attribute::from((k.as_bytes(), v.as_bytes())));
        }
        let c = self.get_classes();
        if !c.is_empty() {
            bs.push_attribute(Attribute::from((
                "class".as_bytes(),
                c.join(" ").as_bytes(),
            )));
        }
        bs
    }
}

impl TryFrom<&BytesStart<'_>> for SvgElement {
    type Error = SvgdxError;

    /// Build a `SvgElement` from a `BytesStart` value. Failures here are are low-level
    /// XML type errors (e.g. bad attribute names, non-UTF8) rather than anything
    /// semantic about svgdx / svg formats.
    fn try_from(e: &BytesStart) -> Result<Self> {
        let elem_name: String =
            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");

        let attrs: Result<Vec<(String, String)>> = e
            .attributes()
            .map(move |a| {
                let aa = a.map_err(SvgdxError::from_err)?;
                let key = String::from_utf8(aa.key.into_inner().to_vec())?;
                let value = aa
                    .unescape_value()
                    .map_err(SvgdxError::from_err)?
                    .into_owned();
                Ok((key, value))
            })
            .collect();
        Ok(Self::new(&elem_name, &attrs?))
    }
}

impl TryFrom<InputEvent> for SvgElement {
    type Error = SvgdxError;

    fn try_from(ev: InputEvent) -> Result<Self> {
        match ev.event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let mut element = SvgElement::try_from(e)?;
                element.original = String::from_utf8(e.to_owned().to_vec()).expect("utf8");
                element.set_indent(ev.indent);
                element.set_src_line(ev.line);
                element.set_order_index(&OrderIndex::new(ev.index));
                Ok(element)
            }
            _ => Err(SvgdxError::DocumentError(format!(
                "Expected Start or Empty event, got {:?}",
                ev.event
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eventlist_minimal() {
        let input = r#"<svg></svg>"#;
        let el = InputList::from_str(input).unwrap();
        assert_eq!(el.events.len(), 2);
        assert_eq!(el.events[0].line, 1);
        assert_eq!(el.events[0].event, Event::Start(BytesStart::new("svg")));
        assert_eq!(el.events[1].line, 1);
        assert_eq!(el.events[1].event, Event::End(BytesEnd::new("svg")));
    }

    #[test]
    fn test_eventlist_indent() {
        let input = r#"<svg>
        </svg>"#;
        let el = InputList::from_str(input).unwrap();
        assert_eq!(el.events.len(), 3);
        assert_eq!(el.events[0].line, 1);
        assert_eq!(el.events[0].indent, 0);
        assert_eq!(el.events[0].event, Event::Start(BytesStart::new("svg")));
        // Multi-line events (e.g. text here) store starting line number
        assert_eq!(el.events[1].line, 1);
        assert_eq!(
            el.events[1].event,
            Event::Text(BytesText::new("\n        "))
        );
        assert_eq!(el.events[2].line, 2);
        assert_eq!(el.events[2].indent, 8);
        assert_eq!(el.events[2].event, Event::End(BytesEnd::new("svg")));
    }

    #[test]
    fn test_outputlist_write_to() {
        let input = r#"<svg><rect width="100" height="100"/></svg>"#;
        let input_list = InputList::from_str(input).unwrap();
        let output_list: OutputList = input_list.into();

        let mut cursor = Cursor::new(Vec::new());
        output_list.write_to(&mut cursor).unwrap();

        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_outputlist_partition_rect() {
        let input = r#"<svg><rect/><circle/><ellipse/></svg>"#;
        let input_list = InputList::from_str(input).unwrap();
        let output_list: OutputList = input_list.into();

        let (before, pivot, after) = output_list.partition("rect");
        assert!(pivot.is_some());
        assert_eq!(
            pivot.unwrap(),
            OutputEvent::Empty(SvgElement::new("rect", &[]))
        );

        let mut cursor = Cursor::new(Vec::new());
        before.write_to(&mut cursor).unwrap();
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, "<svg>");

        let mut cursor = Cursor::new(Vec::new());
        after.write_to(&mut cursor).unwrap();
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, "<circle/><ellipse/></svg>");
    }

    #[test]
    fn test_outputlist_partition_circle() {
        let input = r#"<svg><rect/><circle/><ellipse/></svg>"#;
        let input_list = InputList::from_str(input).unwrap();
        let output_list: OutputList = input_list.into();

        let (before, pivot, after) = output_list.partition("circle");
        assert!(pivot.is_some());
        assert_eq!(
            pivot.unwrap(),
            OutputEvent::Empty(SvgElement::new("circle", &[]))
        );

        let mut cursor = Cursor::new(Vec::new());
        before.write_to(&mut cursor).unwrap();
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, "<svg><rect/>");

        let mut cursor = Cursor::new(Vec::new());
        after.write_to(&mut cursor).unwrap();
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, "<ellipse/></svg>");
    }

    #[test]
    fn test_outputlist_partition_ellipse() {
        let input = r#"<svg><rect/><circle/><ellipse/></svg>"#;
        let input_list = InputList::from_str(input).unwrap();
        let output_list: OutputList = input_list.into();

        let (before, pivot, after) = output_list.partition("ellipse");
        assert!(pivot.is_some());
        assert_eq!(
            pivot.unwrap(),
            OutputEvent::Empty(SvgElement::new("ellipse", &[]))
        );

        let mut cursor = Cursor::new(Vec::new());
        before.write_to(&mut cursor).unwrap();
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, "<svg><rect/><circle/>");

        let mut cursor = Cursor::new(Vec::new());
        after.write_to(&mut cursor).unwrap();
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(result, "</svg>");
    }
}
