use crate::element::SvgElement;

use std::io::{BufRead, Write};

use lazy_regex::regex;
use quick_xml::events::{BytesText, Event};
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
pub struct InputEvent<'a> {
    pub event: Event<'a>,
    pub index: usize,
    pub line: usize,
    pub indent: usize,
}

impl Clone for InputEvent<'_> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone().into_owned(),
            index: self.index,
            line: self.line,
            indent: self.indent,
        }
    }
}

impl<'a> InputEvent<'a> {
    fn into_owned(self) -> InputEvent<'static> {
        InputEvent {
            event: self.event.into_owned(),
            index: self.index,
            line: self.line,
            indent: self.indent,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventList<'a> {
    pub events: Vec<InputEvent<'a>>,
}

impl From<&str> for EventList<'_> {
    fn from(value: &str) -> Self {
        Self::from_str(value).expect("failed to parse string")
    }
}

impl From<Event<'_>> for EventList<'_> {
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

impl From<SvgEvent> for InputEvent<'_> {
    fn from(value: SvgEvent) -> Self {
        InputEvent::from(Event::from(value))
    }
}

impl From<SvgEvent> for EventList<'_> {
    fn from(value: SvgEvent) -> Self {
        Event::from(value).into()
    }
}

impl From<Vec<InputEvent<'_>>> for EventList<'_> {
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

impl From<Vec<Event<'_>>> for EventList<'_> {
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

impl<'a> IntoIterator for EventList<'a> {
    type Item = InputEvent<'a>;
    type IntoIter = std::vec::IntoIter<InputEvent<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl From<Event<'_>> for InputEvent<'_> {
    fn from(value: Event) -> Self {
        Self {
            event: value.into_owned(),
            index: 0,
            line: 0,
            indent: 0,
        }
    }
}

impl EventList<'_> {
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

    pub fn push<'a>(&mut self, ev: impl Into<InputEvent<'a>>) {
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

        let blank_line_remover = regex!("\n[ \t]+\n");
        for event_pos in &self.events {
            // trim trailing whitespace.
            // just using `trim_end()` on Text events won't work
            // as Text event may be followed by a Start/Empty event.
            // blank lines *within* Text can be trimmed.
            let mut event = event_pos.event.clone();
            if let Event::Text(t) = event {
                let mut content = String::from_utf8(t.as_ref().to_vec())?;
                content = blank_line_remover.replace_all(&content, "\n\n").to_string();
                event = Event::Text(BytesText::new(&content).into_owned());
            }
            writer.write_event(event)?;
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
