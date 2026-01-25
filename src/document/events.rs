use crate::errors::{Error, Result};
use crate::types::OrderIndex;

use std::io::{BufReader, Cursor};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    Empty(RawElement),
    Start(RawElement),
    End(String),
    Comment(String),
    Text(String),
    CData(String),
    Other(super::RawXmlEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventMeta {
    pub(super) index: usize,
    pub(super) order: OrderIndex,
    pub(super) line: usize,
    pub(super) indent: usize,
    pub(super) alt_idx: Option<usize>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub event: EventKind,
    pub meta: EventMeta,
}

impl InputEvent {
    pub fn text_string(&self) -> Option<String> {
        match &self.event {
            EventKind::Text(content) => Some(content.to_owned()),
            _ => None,
        }
    }

    pub fn cdata_string(&self) -> Option<String> {
        match &self.event {
            EventKind::CData(content) => Some(content.to_owned()),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawElement(pub String, pub Vec<(String, String)>);

impl RawElement {
    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn get_attrs(&self) -> &Vec<(String, String)> {
        &self.1
    }

    pub fn get_attrs_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.1
    }
}

impl std::fmt::Display for RawElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        for (k, v) in &self.1 {
            write!(f, r#" {}="{}""#, k, v)?;
        }
        Ok(())
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

#[derive(Debug, Default, Clone)]
pub struct OutputList {
    pub events: Vec<InputEvent>,
}

impl From<Vec<EventKind>> for OutputList {
    fn from(value: Vec<EventKind>) -> Self {
        Self {
            events: value
                .into_iter()
                .map(|ev| InputEvent {
                    event: ev,
                    meta: EventMeta::default(),
                })
                .collect(),
        }
    }
}

impl Extend<EventKind> for OutputList {
    fn extend<T: IntoIterator<Item = EventKind>>(&mut self, iter: T) {
        for ev in iter {
            self.push(ev);
        }
    }
}

impl<'a> Extend<&'a InputEvent> for OutputList {
    fn extend<T: IntoIterator<Item = &'a InputEvent>>(&mut self, iter: T) {
        self.events.extend(iter.into_iter().cloned());
    }
}

impl Extend<InputEvent> for OutputList {
    fn extend<T: IntoIterator<Item = InputEvent>>(&mut self, iter: T) {
        self.events.extend(iter);
    }
}

impl From<InputList> for OutputList {
    fn from(value: InputList) -> Self {
        Self {
            events: value.events,
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

    pub fn iter(&self) -> impl Iterator<Item = &InputEvent> + '_ {
        self.events.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut InputEvent> + '_ {
        self.events.iter_mut()
    }

    pub fn push(&mut self, ev: impl Into<EventKind>) {
        let input_ev = InputEvent {
            event: ev.into(),
            // TODO: if we don't actually use meta, should OutputList just be Vec<EventKind>?
            meta: EventMeta::default(),
        };
        self.events.push(input_ev);
    }

    /// Split an `OutputList` into (up to) 3 parts: before, pivot, after.
    pub fn partition(&self, name: &str) -> (Self, Option<InputEvent>, Self) {
        let mut before = vec![];
        let mut pivot = None;
        let mut after = vec![];
        for output_ev in self.events.clone() {
            if pivot.is_some() {
                after.push(output_ev);
            } else {
                match &output_ev.event {
                    EventKind::Start(e) | EventKind::Empty(e) => {
                        if e.0 == name {
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
    type Item = EventKind;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.events
            .iter()
            .map(|ev| ev.event.clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outputlist_partition_rect() {
        let input = r#"<svg><rect/><circle/><ellipse/></svg>"#;
        let input_list = InputList::from_str(input).unwrap();
        let output_list: OutputList = input_list.into();

        let (before, pivot, after) = output_list.partition("rect");
        assert!(pivot.is_some());
        assert_eq!(
            pivot.unwrap().event,
            EventKind::Empty(RawElement("rect".into(), vec![])),
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
            pivot.unwrap().event,
            EventKind::Empty(RawElement("circle".into(), vec![]))
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
            pivot.unwrap().event,
            EventKind::Empty(RawElement("ellipse".into(), vec![]))
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
