use super::{EventKind, InputEvent, InputList, RawElement};
use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::types::OrderIndex;

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
            EventKind::Start(_) => {
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
            EventKind::Empty(_) => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    Error::Document(format!(
                        "could not extract element at line {}",
                        input_ev.meta.line
                    ))
                })?;
                event_element.set_event_range((input_ev.meta.index, input_ev.meta.index));
                tags.push(Tag::Leaf(event_element, None));
            }
            EventKind::Comment(content) => {
                tags.push(Tag::Comment(
                    input_ev.meta.order.clone(),
                    content.clone(),
                    None,
                ));
            }
            EventKind::Text(content) => {
                if let Some(t) = tags.last_mut() {
                    t.set_text(content.clone())
                } else {
                    tags.push(Tag::Text(input_ev.meta.order.clone(), content.clone()));
                }
            }
            EventKind::CData(content) => {
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

impl From<SvgElement> for RawElement {
    fn from(value: SvgElement) -> Self {
        Self(value.name().to_owned(), value.get_full_attrs().to_vec())
    }
}

impl TryFrom<InputEvent> for SvgElement {
    type Error = Error;

    fn try_from(ev: InputEvent) -> Result<Self> {
        match ev.event {
            EventKind::Start(el) | EventKind::Empty(el) => {
                let mut element = SvgElement::new(&el.0, &el.1);
                element.original = el.to_string();
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
