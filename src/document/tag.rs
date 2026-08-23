use super::{EventKind, InputEvent, InputList, RawElement, Spacing};
use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::types::OrderIndex;

#[derive(Debug, Clone)]
pub enum Tag {
    /// Represents a Start..End block and all events in between
    Compound(SvgElement, Spacing), // element, leading spacing
    /// Represents a single Empty element
    Leaf(SvgElement, Spacing), // element, leading spacing
    /// Comment tags need the source line and ordering information because they don't
    /// have a backing `SvgElement`, but still participate in same/next-line spacing.
    Comment(OrderIndex, usize, String, Spacing), // comment, line number, leading spacing
    /// Tag::Spacing only occurs when a sliced event list ends with whitespace after
    /// its last real tag. We keep it as a tag so containers can still emit that
    /// spacing before a later outer closing tag such as `</g>` or `</defs>`.
    Spacing(OrderIndex, Spacing),
}

impl Tag {
    fn set_spacing(&mut self, spacing: Spacing) {
        match self {
            Tag::Compound(_, lead) => lead.merge(spacing),
            Tag::Leaf(_, lead) => lead.merge(spacing),
            Tag::Comment(_, _, _, lead) => lead.merge(spacing),
            Tag::Spacing(_, current) => current.merge(spacing),
        }
    }

    pub fn spacing(&self) -> &Spacing {
        match self {
            Tag::Compound(_, lead) => lead,
            Tag::Leaf(_, lead) => lead,
            Tag::Comment(_, _, _, lead) => lead,
            Tag::Spacing(_, current) => current,
        }
    }

    pub fn src_line(&self) -> usize {
        match self {
            Tag::Compound(el, _) => el.src_line,
            Tag::Leaf(el, _) => el.src_line,
            Tag::Comment(_, line, _, _) => *line,
            Tag::Spacing(_, _) => 0,
        }
    }

    pub fn get_order_index(&self) -> OrderIndex {
        match self {
            Tag::Compound(el, _) => el.order_index.clone(),
            Tag::Leaf(el, _) => el.order_index.clone(),
            Tag::Comment(oi, _, _, _) => oi.clone(),
            Tag::Spacing(oi, _) => oi.clone(),
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
    let mut pending_spacing = Spacing::default();
    let mut pending_spacing_order = None;

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
                tags.push(Tag::Compound(event_element, pending_spacing.take()));
            }
            EventKind::Empty(_) => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    Error::Document(format!(
                        "could not extract element at line {}",
                        input_ev.meta.line
                    ))
                })?;
                event_element.set_event_range((input_ev.meta.index, input_ev.meta.index));
                tags.push(Tag::Leaf(event_element, pending_spacing.take()));
            }
            EventKind::Comment(content) => {
                tags.push(Tag::Comment(
                    input_ev.meta.order.clone(),
                    input_ev.meta.line,
                    content.clone(),
                    pending_spacing.take(),
                ));
            }
            EventKind::Text(content) | EventKind::CData(content) => {
                if let Some(spacing) = Spacing::from_text(content) {
                    pending_spacing.merge(spacing);
                    pending_spacing_order = Some(input_ev.meta.order.clone());
                }
            }
            _ => {
                // This would include Event::End, as well as PI, DocType, etc.
                // Specifically End shouldn't be seen due to alt_idx scan-ahead.
            }
        }
    }

    // Update tag spacing based on whether tags are from the same line or not
    // Skips first tag because needs a previous line to compare.
    for idx in 1..tags.len() {
        let prev_line = tags[idx - 1].src_line();
        let cur_line = tags[idx].src_line();
        if prev_line == cur_line {
            tags[idx].set_spacing(Spacing::Inline);
        } else if prev_line < cur_line {
            tags[idx].set_spacing(Spacing::LineBreak);
        }
    }

    if let Some(order) = pending_spacing_order
        && !pending_spacing.is_empty()
    {
        tags.push(Tag::Spacing(order, pending_spacing));
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
