use crate::element::SvgElement;

use std::io::{BufRead, Write};

use lazy_regex::regex;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use anyhow::{bail, Result};

pub enum SvgEvent {
    Comment(String),
    Text(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub event: Event<'static>,
    pub index: usize,
    pub line: usize,
    pub indent: usize,
}

impl Clone for InputEvent {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            index: self.index,
            line: self.line,
            indent: self.indent,
        }
    }
}

impl InputEvent {
    fn into_owned(self) -> InputEvent {
        InputEvent {
            event: self.event,
            index: self.index,
            line: self.line,
            indent: self.indent,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventList {
    pub events: Vec<InputEvent>,
}

impl From<&str> for EventList {
    fn from(value: &str) -> Self {
        Self::from_str(value).expect("failed to parse string")
    }
}

impl From<Event<'_>> for EventList {
    fn from(value: Event) -> Self {
        Self {
            events: vec![InputEvent {
                event: value.into_owned(),
                index: 0,
                line: 0,
                indent: 0,
            }],
        }
    }
}

impl From<SvgEvent> for InputEvent {
    fn from(value: SvgEvent) -> Self {
        InputEvent::from(Event::from(value))
    }
}

impl From<SvgEvent> for EventList {
    fn from(value: SvgEvent) -> Self {
        Event::from(value).into()
    }
}

impl From<Vec<InputEvent>> for EventList {
    fn from(value: Vec<InputEvent>) -> Self {
        Self {
            events: value
                .into_iter()
                .map(|v| InputEvent {
                    event: v.event.into_owned(),
                    index: v.index,
                    line: v.line,
                    indent: v.indent,
                })
                .collect(),
        }
    }
}

impl From<Vec<Event<'_>>> for EventList {
    fn from(value: Vec<Event>) -> Self {
        Self {
            events: value
                .into_iter()
                .map(|v| InputEvent {
                    event: v.into_owned(),
                    index: 0,
                    line: 0,
                    indent: 0,
                })
                .collect(),
        }
    }
}

impl IntoIterator for EventList {
    type Item = InputEvent;
    type IntoIter = std::vec::IntoIter<InputEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl From<Event<'_>> for InputEvent {
    fn from(value: Event) -> Self {
        Self {
            event: value.into_owned(),
            index: 0,
            line: 0,
            indent: 0,
        }
    }
}

impl Default for EventList {
    fn default() -> Self {
        Self::new()
    }
}

impl EventList {
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
        self.events.push(ev.clone().into_owned());
    }

    pub fn extend(&mut self, other: &EventList) {
        for ev in other.iter() {
            self.push(ev.event.clone());
        }
    }

    pub fn from_reader(reader: &mut dyn BufRead) -> Result<Self> {
        let mut reader = Reader::from_reader(reader);

        let mut events = Vec::new();
        let mut buf = Vec::new();

        let mut line_count = 1;
        let mut indent = 0;
        let mut index = 0;
        loop {
            let ev = reader.read_event_into(&mut buf);
            if let Ok(ok_ev) = ev.clone() {
                line_count += &ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count();
            }
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
                        line: line_count,
                        indent,
                    });
                }
                Ok(e) => events.push(InputEvent {
                    event: e.clone().into_owned(),
                    index,
                    line: line_count,
                    indent,
                }),
                Err(e) => bail!("XML error near line {}: {:?}", line_count, e),
            };

            index += 1;
            buf.clear();
        }

        Ok(Self { events })
    }

    pub fn from_str(s: impl Into<String>) -> Result<Self> {
        let s: String = s.into();
        let mut reader = Reader::from_str(&s);
        let mut events = Vec::new();

        // TODO: remove duplication between this and `from_reader`
        let mut line_count = 1;
        let mut indent = 0;
        let mut index = 0;
        loop {
            let ev = reader.read_event();
            if let Ok(ok_ev) = ev.clone() {
                line_count += &ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count();
            }
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Text(t)) => {
                    let t_str = String::from_utf8(t.to_vec())?;
                    if let Some((_, rest)) = t_str.rsplit_once('\n') {
                        indent = rest.len() - rest.trim_end_matches(' ').len();
                    }

                    events.push(InputEvent {
                        event: ev.expect("match").clone().into_owned(),
                        index,
                        line: line_count,
                        indent,
                    });
                }
                Ok(e) => events.push(InputEvent {
                    event: e.clone().into_owned(),
                    index,
                    line: line_count,
                    indent,
                }),
                Err(e) => bail!("XML error near line {}: {:?}", line_count, e),
            }

            index += 1;
        }
        Ok(Self { events })
    }

    pub fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            events: self.events[start..end].to_vec(),
        }
    }

    pub fn write_to(&self, writer: &mut dyn Write) -> Result<()> {
        let mut writer = Writer::new(writer);

        // Separate buffer for coalescing text events
        let mut text_buf = String::new();

        let blank_line_remover = regex!("[ \t]+\n");
        for event_pos in &self.events {
            // trim trailing whitespace.
            // just using `trim_end()` on Text events won't work
            // as Text event may be followed by a Start/Empty event.
            // blank lines *within* Text can be trimmed.
            let event = event_pos.event.clone();
            if let Event::Text(ref t) = event {
                let content = String::from_utf8(t.as_ref().to_vec())?;
                text_buf.push_str(&content);
                continue;
            } else if !text_buf.is_empty() {
                let content = blank_line_remover.replace_all(&text_buf, "\n").to_string();
                let text_event = Event::Text(BytesText::new(&content).into_owned());
                text_buf.clear();
                writer.write_event(text_event)?;
            }
            writer.write_event(event)?;
        }
        // re-add any trailing text
        if !text_buf.is_empty() {
            let content = blank_line_remover.replace_all(&text_buf, "\n").to_string();
            let text_event = Event::Text(BytesText::new(&content).into_owned());
            writer.write_event(text_event)?;
        }
        Ok(())
    }

    /// Split an `EventList` into (up to) 3 parts: before, pivot, after.
    pub fn partition(&self, name: &str) -> (Self, Option<InputEvent>, Self) {
        let mut before = vec![];
        let mut pivot = None;
        let mut after = vec![];
        for input_ev in self.events.clone() {
            if pivot.is_some() {
                after.push(input_ev);
            } else {
                match input_ev.event {
                    Event::Start(ref e) | Event::Empty(ref e) => {
                        let elem_name: String =
                            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");
                        if elem_name == name {
                            pivot = Some(input_ev);
                        } else {
                            before.push(input_ev);
                        }
                    }
                    _ => before.push(input_ev),
                }
            }
        }

        (Self { events: before }, pivot, Self { events: after })
    }
}

impl<'a> From<SvgEvent> for Event<'a> {
    fn from(svg_ev: SvgEvent) -> Event<'a> {
        match svg_ev {
            SvgEvent::Empty(e) => Event::Empty(e.into()),
            SvgEvent::Start(e) => Event::Start(e.into()),
            SvgEvent::Comment(t) => Event::Comment(BytesText::from_escaped(t)),
            SvgEvent::Text(t) => Event::Text(BytesText::from_escaped(t)),
            SvgEvent::End(name) => Event::End(BytesEnd::new(name)),
        }
    }
}

impl From<SvgElement> for BytesStart<'static> {
    fn from(e: SvgElement) -> BytesStart<'static> {
        let mut bs = BytesStart::new(e.name);
        for (k, v) in e.attrs {
            bs.push_attribute(Attribute::from((k.as_bytes(), v.as_bytes())));
        }
        if !e.classes.is_empty() {
            bs.push_attribute(Attribute::from((
                "class".as_bytes(),
                e.classes
                    .into_iter()
                    .collect::<Vec<String>>()
                    .join(" ")
                    .as_bytes(),
            )));
        }
        bs
    }
}

impl TryFrom<&BytesStart<'_>> for SvgElement {
    type Error = anyhow::Error;

    /// Build a `SvgElement` from a `BytesStart` value. Failures here are are low-level
    /// XML type errors (e.g. bad attribute names, non-UTF8) rather than anything
    /// semantic about svgdx / svg formats.
    fn try_from(e: &BytesStart) -> Result<Self, Self::Error> {
        let elem_name: String =
            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");

        let attrs: Result<Vec<(String, String)>, Self::Error> = e
            .attributes()
            .map(move |a| {
                let aa = a?;
                let key = String::from_utf8(aa.key.into_inner().to_vec())?;
                let value = aa.unescape_value()?.into_owned();
                Ok((key, value))
            })
            .collect();
        Ok(Self::new(&elem_name, &attrs?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eventlist_minimal() {
        let input = r#"<svg></svg>"#;
        let el = EventList::from_str(input).unwrap();
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
        let el = EventList::from_str(input).unwrap();
        assert_eq!(el.events.len(), 3);
        assert_eq!(el.events[0].line, 1);
        assert_eq!(el.events[0].indent, 0);
        assert_eq!(el.events[0].event, Event::Start(BytesStart::new("svg")));
        assert_eq!(el.events[1].line, 2);
        assert_eq!(
            el.events[1].event,
            Event::Text(BytesText::new("\n        "))
        );
        assert_eq!(el.events[2].line, 2);
        assert_eq!(el.events[2].indent, 8);
        assert_eq!(el.events[2].event, Event::End(BytesEnd::new("svg")));
    }
}
