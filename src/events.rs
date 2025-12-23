use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::types::OrderIndex;

use std::io::{BufRead, BufReader, Cursor, Write};
use std::str::FromStr;

use quick_xml::escape::partial_escape;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event as XmlEvent};
use quick_xml::{Reader, Writer};

#[derive(Clone, Debug, PartialEq, Eq)]
enum EventKind {
    Empty {
        name: String,
        attributes: Attributes,
    },
    Start {
        name: String,
        attributes: Attributes,
    },
    End {
        name: String,
    },
    Comment {
        content: String,
    },
    Text {
        content: String,
    },
    CData {
        content: String,
    },
    Other {
        event: XmlEvent<'static>,
    },
}

impl EventKind {
    pub fn is_eof(&self) -> bool {
        matches!(self, EventKind::Other { event } if matches!(event, XmlEvent::Eof))
    }
}

impl TryFrom<XmlEvent<'_>> for EventKind {
    type Error = Error;
    fn try_from(event: XmlEvent) -> Result<Self> {
        let res = match event {
            XmlEvent::Empty(bs) => {
                let name = String::from_utf8(bs.name().into_inner().to_vec()).expect("utf8");
                EventKind::Empty {
                    name,
                    attributes: bs.try_into()?,
                }
            }
            XmlEvent::Start(bs) => {
                let name = String::from_utf8(bs.name().into_inner().to_vec()).expect("utf8");
                EventKind::Start {
                    name,
                    attributes: bs.try_into()?,
                }
            }
            XmlEvent::End(e) => {
                let name = String::from_utf8(e.name().into_inner().to_vec()).expect("utf8");
                EventKind::End { name }
            }
            XmlEvent::Text(t) => {
                let content = String::from_utf8(t.into_inner().to_vec()).expect("utf8");
                EventKind::Text { content }
            }
            XmlEvent::CData(c) => {
                let content = String::from_utf8(c.into_inner().to_vec()).expect("utf8");
                EventKind::CData { content }
            }
            XmlEvent::Comment(c) => {
                let content = String::from_utf8(c.into_inner().to_vec()).expect("utf8");
                EventKind::Comment { content }
            }
            other => EventKind::Other {
                event: other.into_owned(),
            },
        };
        Ok(res)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEvent {
    event: EventKind,
    meta: EventMeta,
}

impl InputEvent {
    pub fn text_string(&self) -> Option<String> {
        match &self.event {
            EventKind::Text { content } => Some(content.to_owned()),
            _ => None,
        }
    }

    pub fn cdata_string(&self) -> Option<String> {
        match &self.event {
            EventKind::CData { content } => Some(content.to_owned()),
            _ => None,
        }
    }

    pub fn with_base_index(&self, order: &OrderIndex) -> Self {
        Self {
            meta: EventMeta {
                order: order.with_sub_index(&self.meta.order),
                ..self.meta.clone()
            },
            ..self.clone()
        }
    }

    pub fn set_span(&mut self, index: usize, alt_idx: usize) {
        self.meta.index = index;
        self.meta.alt_idx = Some(alt_idx);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EventMeta {
    index: usize,
    order: OrderIndex,
    line: usize,
    indent: usize,
    alt_idx: Option<usize>,
}

impl Default for EventMeta {
    fn default() -> Self {
        Self {
            index: 0,
            order: OrderIndex::new(0),
            line: 0,
            indent: 0,
            alt_idx: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Attributes(Vec<(String, String)>);

impl TryFrom<BytesStart<'_>> for Attributes {
    type Error = Error;

    fn try_from(e: BytesStart) -> Result<Self> {
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
        Ok(Self(attrs?))
    }
}

impl TryFrom<XmlEvent<'_>> for InputEvent {
    type Error = Error;

    fn try_from(value: XmlEvent) -> Result<Self> {
        Ok(Self {
            event: match value {
                XmlEvent::Empty(e) => {
                    let name = String::from_utf8(e.name().into_inner().to_vec()).expect("utf8");
                    EventKind::Empty {
                        name,
                        attributes: e.try_into()?,
                    }
                }
                XmlEvent::Start(e) => {
                    let name = String::from_utf8(e.name().into_inner().to_vec()).expect("utf8");
                    EventKind::Start {
                        name,
                        attributes: e.try_into()?,
                    }
                }
                XmlEvent::End(e) => {
                    let name = String::from_utf8(e.name().into_inner().to_vec()).expect("utf8");
                    EventKind::End { name }
                }
                XmlEvent::Text(t) => {
                    let content = String::from_utf8(t.into_inner().to_vec()).expect("utf8");
                    EventKind::Text { content }
                }
                XmlEvent::CData(c) => {
                    let content = String::from_utf8(c.into_inner().to_vec()).expect("utf8");
                    EventKind::CData { content }
                }
                XmlEvent::Comment(c) => {
                    let content = String::from_utf8(c.into_inner().to_vec()).expect("utf8");
                    EventKind::Comment { content }
                }
                _ => EventKind::Other {
                    event: value.into_owned(),
                },
            },
            meta: EventMeta::default(),
        })
    }
}

impl From<OutputEvent> for InputEvent {
    fn from(value: OutputEvent) -> Self {
        let meta = EventMeta::default();
        match value {
            OutputEvent::Comment(c) => InputEvent {
                event: EventKind::Comment { content: c },
                meta,
            },
            OutputEvent::Text(t) => InputEvent {
                event: EventKind::Text { content: t },
                meta,
            },
            OutputEvent::CData(t) => InputEvent {
                event: EventKind::CData { content: t },
                meta,
            },
            OutputEvent::Start(el) => InputEvent {
                event: EventKind::Start {
                    name: el.name().to_owned(),
                    attributes: Attributes(el.get_full_attrs().to_vec()),
                },
                meta: EventMeta {
                    line: el.src_line,
                    indent: el.indent,
                    order: el.order_index,
                    index: el.event_range.unwrap_or((0, 0)).0,
                    alt_idx: el.event_range.map(|(_, alt)| alt),
                },
            },
            OutputEvent::Empty(el) => InputEvent {
                event: EventKind::Empty {
                    name: el.name().to_owned(),
                    attributes: Attributes(el.get_full_attrs().to_vec()),
                },
                meta: EventMeta {
                    line: el.src_line,
                    indent: el.indent,
                    order: el.order_index,
                    index: el.event_range.unwrap_or((0, 0)).0,
                    alt_idx: el.event_range.map(|(_, alt)| alt),
                },
            },
            OutputEvent::End(name) => InputEvent {
                event: EventKind::End { name },
                meta,
            },
            OutputEvent::Other(event) => InputEvent {
                event: EventKind::Other { event },
                meta,
            },
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct InputList {
    pub events: Vec<InputEvent>,
}

impl From<&[InputEvent]> for InputList {
    fn from(value: &[InputEvent]) -> Self {
        Self {
            events: value.to_vec(),
        }
    }
}

impl From<Vec<InputEvent>> for InputList {
    fn from(value: Vec<InputEvent>) -> Self {
        Self { events: value }
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
    type Err = Error;

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
                EventKind::Text { content } => {
                    let mut t_str = content.clone();
                    if let Some((_, rest)) = t_str.rsplit_once('\n') {
                        t_str = rest.to_string();
                    }
                    indent = t_str.len() - t_str.trim_end_matches(' ').len();
                    meta.indent = indent;
                    events.push(InputEvent {
                        event: EventKind::Text { content },
                        meta,
                    });
                    order.step();
                }
                EventKind::Start { name, attributes } => {
                    events.push(InputEvent {
                        event: EventKind::Start { name, attributes },
                        meta,
                    });
                    event_idx_stack.push(index);
                    order.down();
                }
                EventKind::End { name } => {
                    let start_idx = event_idx_stack.pop();
                    if let Some(start_idx) = start_idx {
                        events[start_idx].meta.alt_idx = Some(index);
                    }
                    order.up();
                    meta.alt_idx = start_idx;
                    events.push(InputEvent {
                        event: EventKind::End { name },
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

    pub fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            events: self.events[start..end].to_vec(),
        }
    }

    pub fn rebase_index(&mut self, oi_base: OrderIndex) {
        // replace all order indices of events such that the first event is `oi_base`
        let mut oi = oi_base.clone();
        for ev in &mut self.events {
            match &ev.event {
                EventKind::Start { .. } => {
                    ev.meta.order = oi.clone();
                    oi.down();
                }
                // end events will have the same order index as the start event,
                // but should never have their order index used...
                EventKind::End { .. } => {
                    oi.up();
                    ev.meta.order = oi.clone();
                    oi.step();
                }
                _ => {
                    ev.meta.order = oi.clone();
                    oi.step();
                }
            }
        }
    }

    pub fn rebase_under(&mut self, oi_base: OrderIndex) {
        // replace all order indices of events to be under `oi_base`
        let mut oi = oi_base.clone();
        oi.down();
        self.rebase_index(oi);
    }
}

#[derive(Debug, Clone)]
pub enum Tag {
    /// Represents a Start..End block and all events in between
    Compound(SvgElement, Option<String>), // element, tail
    /// Represents a single Empty element
    Leaf(SvgElement, Option<String>), // element, tail
    Comment(OrderIndex, String, Option<String>), // comment, tail
    Text(OrderIndex, String),
    CData(OrderIndex, String),
}

impl Tag {
    fn set_text(&mut self, text: String) {
        match self {
            Tag::Compound(_, tail) => *tail = Some(text),
            Tag::Leaf(_, tail) => *tail = Some(text),
            Tag::Comment(_, _, tail) => *tail = Some(text),
            _ => {}
        }
    }

    pub fn get_order_index(&self) -> OrderIndex {
        match self {
            Tag::Compound(el, _) => el.order_index.clone(),
            Tag::Leaf(el, _) => el.order_index.clone(),
            Tag::Comment(oi, _, _) => oi.clone(),
            Tag::Text(oi, _) => oi.clone(),
            Tag::CData(oi, _) => oi.clone(),
        }
    }

    pub fn get_element_mut(&mut self) -> Option<&mut SvgElement> {
        match self {
            Tag::Compound(el, _) => Some(el),
            Tag::Leaf(el, _) => Some(el),
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
            EventKind::Start { .. } => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    Error::Document(format!(
                        "could not extract element at line {}",
                        input_ev.meta.line
                    ))
                })?;
                if let Some(alt_idx) = input_ev.meta.alt_idx {
                    event_element.set_event_range((input_ev.meta.index, alt_idx));
                    // Scan ahead to the end of this element, matching alt_idx.
                    // Note when called recursively on a subset of events, alt_idx
                    // won't be the same as next_idx, so we need to scan rather than
                    // just setting ev_idx = alt_idx + 1.
                    for next_idx in ev_idx..events.len() {
                        if events.events[next_idx].meta.index == alt_idx {
                            ev_idx = next_idx + 1; // skip the End event itself
                            break;
                        }
                    }
                } // TODO: else warning message
                tags.push(Tag::Compound(event_element, None));
            }
            EventKind::Empty { .. } => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    Error::Document(format!(
                        "could not extract element at line {}",
                        input_ev.meta.line
                    ))
                })?;
                event_element.set_event_range((input_ev.meta.index, input_ev.meta.index));
                tags.push(Tag::Leaf(event_element, None));
            }
            EventKind::Comment { content } => {
                tags.push(Tag::Comment(
                    input_ev.meta.order.clone(),
                    content.clone(),
                    None,
                ));
            }
            EventKind::Text { content } => {
                if let Some(t) = tags.last_mut() {
                    t.set_text(content.clone())
                } else {
                    tags.push(Tag::Text(input_ev.meta.order.clone(), content.clone()));
                }
            }
            EventKind::CData { content } => {
                if let Some(t) = tags.last_mut() {
                    t.set_text(content.clone())
                } else {
                    tags.push(Tag::CData(input_ev.meta.order.clone(), content.clone()));
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
    Other(XmlEvent<'static>),
}

impl From<InputEvent> for OutputEvent {
    fn from(value: InputEvent) -> Self {
        match value.event {
            EventKind::Empty { name, attributes } => {
                let el = SvgElement::new(&name, &attributes.0);
                OutputEvent::Empty(el)
            }
            EventKind::Start { name, attributes } => {
                let el = SvgElement::new(&name, &attributes.0);
                OutputEvent::Start(el)
            }
            EventKind::End { name } => OutputEvent::End(name),
            EventKind::Text { content } => OutputEvent::Text(content),
            EventKind::CData { content } => OutputEvent::CData(content),
            EventKind::Comment { content } => OutputEvent::Comment(content),
            EventKind::Other { event } => OutputEvent::Other(event),
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

impl Extend<OutputEvent> for OutputList {
    fn extend<T: IntoIterator<Item = OutputEvent>>(&mut self, iter: T) {
        self.events.extend(iter);
    }
}

impl<'a> Extend<&'a OutputEvent> for OutputList {
    fn extend<T: IntoIterator<Item = &'a OutputEvent>>(&mut self, iter: T) {
        self.events.extend(iter.into_iter().cloned());
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

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &OutputEvent> + '_ {
        self.events.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut OutputEvent> + '_ {
        self.events.iter_mut()
    }

    pub fn push(&mut self, ev: impl Into<OutputEvent>) {
        let ev = ev.into();
        self.events.push(ev.clone());
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
                let text_event = OutputEvent::Text(content);
                text_buf.clear();
                writer.write_event(text_event).map_err(Error::from_err)?;
            }
            writer.write_event(event).map_err(Error::from_err)?;
        }
        // re-add any trailing text
        if !text_buf.is_empty() {
            let content = Self::blank_line_remover(&text_buf);
            let text_event = OutputEvent::Text(content);
            writer.write_event(text_event).map_err(Error::from_err)?;
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

impl<'a> From<OutputEvent> for XmlEvent<'a> {
    fn from(svg_ev: OutputEvent) -> XmlEvent<'a> {
        match svg_ev {
            OutputEvent::Empty(e) => XmlEvent::Empty(e.into_bytesstart()),
            OutputEvent::Start(e) => XmlEvent::Start(e.into_bytesstart()),
            OutputEvent::Comment(t) => XmlEvent::Comment(BytesText::from_escaped(t)),
            OutputEvent::Text(t) => XmlEvent::Text(BytesText::from_escaped(partial_escape(t))),
            OutputEvent::CData(t) => XmlEvent::CData(BytesCData::new(t)),
            OutputEvent::End(name) => XmlEvent::End(BytesEnd::new(name)),
            OutputEvent::Other(e) => e,
        }
    }
}

impl SvgElement {
    /// Convert an `SvgElement` into a `BytesStart` value.
    ///
    /// Implemented as a method rather than a `From` impl to keep private
    fn into_bytesstart(self) -> BytesStart<'static> {
        let mut style_attr = None;
        let mut bs = BytesStart::new(self.name().to_owned());
        for (k, v) in self.get_attrs() {
            if k == "style" {
                // generally `style` shouldn't exists in attrs, but it will if
                // malformed, to support round-tripping.
                style_attr = Some(v.clone());
            } else {
                bs.push_attribute(Attribute::from((k.as_bytes(), v.as_bytes())));
            }
        }
        let s = self.get_styles();
        let style = match (style_attr, s.to_string().as_str()) {
            (Some(attr), "") => attr,
            (None, s) if !s.is_empty() => s.to_string(),
            (Some(attr), s) if !s.is_empty() => format!("{attr};{s}"),
            _ => String::new(),
        };
        if !style.is_empty() {
            bs.push_attribute(Attribute::from(("style".as_bytes(), style.as_bytes())));
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
    type Error = Error;

    /// Build a `SvgElement` from a `BytesStart` value. Failures here are are low-level
    /// XML type errors (e.g. bad attribute names, non-UTF8) rather than anything
    /// semantic about svgdx / svg formats.
    fn try_from(e: &BytesStart) -> Result<Self> {
        let elem_name: String =
            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");

        let attrs: Result<Vec<(String, String)>> = e
            .attributes()
            .map(move |a| {
                let aa = a.map_err(Error::from_err)?;
                let key = String::from_utf8(aa.key.into_inner().to_vec())?;
                let value = aa.unescape_value().map_err(Error::from_err)?.into_owned();
                Ok((key, value))
            })
            .collect();
        Ok(Self::new(&elem_name, &attrs?))
    }
}

impl TryFrom<InputEvent> for SvgElement {
    type Error = Error;

    fn try_from(ev: InputEvent) -> Result<Self> {
        match ev.event {
            EventKind::Start { name, attributes } | EventKind::Empty { name, attributes } => {
                let mut element = SvgElement::new(&name, &attributes.0);
                // TODO: reinstate this!!
                // element.original = String::from_utf8(e.to_owned().to_vec()).expect("utf8");
                element.set_indent(ev.meta.indent);
                element.set_src_line(ev.meta.line);
                element.set_order_index(&ev.meta.order);
                Ok(element)
            }
            _ => Err(Error::Document(format!(
                "expected Start or Empty event, got {:?}",
                ev.event
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_eventlist_minimal() {
    //     let input = r#"<svg></svg>"#;
    //     let el = InputList::from_str(input).unwrap();
    //     assert_eq!(el.events.len(), 2);
    //     assert_eq!(el.events[0].meta.line, 1);
    //     assert_eq!(el.events[0].event, XmlEvent::Start(BytesStart::new("svg")));
    //     assert_eq!(el.events[1].meta.line, 1);
    //     assert_eq!(el.events[1].event, XmlEvent::End(BytesEnd::new("svg")));
    // }

    // #[test]
    // fn test_eventlist_indent() {
    //     let input = r#"<svg>
    //     </svg>"#;
    //     let el = InputList::from_str(input).unwrap();
    //     assert_eq!(el.events.len(), 3);
    //     assert_eq!(el.events[0].meta.line, 1);
    //     assert_eq!(el.events[0].meta.indent, 0);
    //     assert_eq!(el.events[0].event, XmlEvent::Start(BytesStart::new("svg")));
    //     // Multi-line events (e.g. text here) store starting line number
    //     assert_eq!(el.events[1].meta.line, 1);
    //     assert_eq!(
    //         el.events[1].event,
    //         XmlEvent::Text(BytesText::new("\n        "))
    //     );
    //     assert_eq!(el.events[2].meta.line, 2);
    //     assert_eq!(el.events[2].meta.indent, 8);
    //     assert_eq!(el.events[2].event, XmlEvent::End(BytesEnd::new("svg")));
    // }

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
