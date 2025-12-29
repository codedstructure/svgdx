use super::SvgElement;
use crate::context::TransformerContext;
use crate::document::{EventKind, OutputList};
use crate::errors::Result;
use crate::geometry::BoundingBox;
use crate::transform::{process_events, EventGen};

/// Container will be used for many elements which contain other elements,
/// but have no independent behaviour, such as defs, linearGradient, etc.
#[derive(Debug, Clone)]
pub struct Container<'a>(pub &'a SvgElement);

impl EventGen for Container<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        if let Some(inner_events) = self.0.inner_events(context) {
            // If there's only text/cdata events, apply to current element and render
            let mut inner_text = None;
            for e in inner_events.iter() {
                if let Some(t) = e.text_string() {
                    if inner_text.is_none() {
                        inner_text = Some(t);
                    }
                } else if let Some(c) = e.cdata_string() {
                    inner_text = Some(c);
                } else {
                    // not text or cdata - abandon the effort and mark as such.
                    inner_text = None;
                    break;
                }
            }
            if let Some(text) = &inner_text {
                let mut el = self.0.clone();
                el.set_attr("text", text);
                if let Some((start, _end)) = self.0.event_range {
                    el.event_range = Some((start, start)); // emulate an Empty element
                }
            }
            if let (true, Some(text)) = (is_graphics_element(self.0), &inner_text) {
                let mut el = self.0.clone();
                el.set_attr("text", text);
                if let Some((start, _end)) = self.0.event_range {
                    el.event_range = Some((start, start)); // emulate an Empty element
                }
                el.generate_events(context)
            } else {
                let mut new_el = self.0.clone();
                let mut bbox = None;

                if is_graphics_element(&new_el) {
                    // TODO: this duplicates part of the `OtherElement::generate_events`
                    // logic; should really be based on graphics vs container element
                    // rather than whether the XML element is empty or not.
                    new_el.resolve_position(context)?; // transmute assumes some of this (e.g. dxy -> dx/dy) has been done
                    new_el.transmute(context)?;
                    bbox = new_el.bbox()?;
                }

                // Special case <svg> elements with an xmlns attribute - passed through
                // transparently, with no bbox calculation.
                if new_el.name() == "svg" && new_el.get_attr("xmlns").is_some() {
                    return Ok((self.0.all_events(context).into(), None));
                }
                new_el.eval_attributes(context)?;
                if context.config.add_metadata {
                    new_el.set_attr("data-src-line", &self.0.src_line.to_string());
                }
                let mut events = OutputList::new();
                events.push(EventKind::Start(new_el.clone().into()));
                let (evlist, inner_bbox) = if inner_text.is_some() {
                    // inner_text implies no processable events; use as-is
                    (inner_events.into(), None)
                } else {
                    context.push_element(self.0);
                    let mut inner_events = inner_events.clone();
                    inner_events.rebase_under(new_el.order_index.clone());
                    let res = process_events(inner_events, context);
                    context.pop_element();
                    res?
                };
                events.extend(evlist);
                events.push(EventKind::End(self.0.name().to_owned()));

                if is_container_element(&new_el) {
                    bbox = inner_bbox;
                }

                if self.0.name() == "defs" {
                    bbox = None;
                } else if bbox.is_some() {
                    new_el.content_bbox = bbox;
                    context.update_element(&new_el);
                }

                Ok((events, bbox))
            }
        } else {
            Ok((OutputList::new(), None))
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupElement<'a>(pub &'a SvgElement);

impl EventGen for GroupElement<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        // since we synthesize the opening element event here, we need to
        // do any required transformations on the <g> itself here.
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context)?;

        // push variables onto the stack
        context.push_element(self.0);

        let (inner, content_bb) = if let Some(inner_events) = self.0.inner_events(context) {
            // get the inner events / bbox first, as some outer element attrs
            // (e.g. `transform` via rotate) may depend on the bbox.
            let mut inner_events = inner_events.clone();
            inner_events.rebase_under(new_el.order_index.clone());
            process_events(inner_events, context).inspect_err(|_| {
                context.pop_element();
            })?
        } else {
            (OutputList::new(), None)
        };

        // pop variables off the stack
        context.pop_element();

        // Need bbox to provide center of rotation
        new_el.content_bbox = content_bb;
        new_el.resolve_position(context)?; // transmute assumes some of this (e.g. dxy -> dx/dy) has been done
        new_el.handle_rotation()?;

        let mut events = OutputList::new();
        if self.0.is_empty_element() {
            events.push(EventKind::Empty(new_el.clone().into()));
        } else {
            let el_name = new_el.name().to_owned();
            events.push(EventKind::Start(new_el.clone().into()));
            events.extend(inner);
            events.push(EventKind::End(el_name));
        }

        context.update_element(&new_el);

        let result_bb = if self.0.name() == "symbol" {
            // symbols have a size which needs storing in context for evaluating
            // bbox of 'use' elements referencing them, but they don't contribute
            // to the parent bbox.
            None
        } else {
            // this handles any `transform` attr. Assumes .content_bbox is set.
            new_el.bbox()?
        };
        Ok((events, result_bb))
    }
}

/// See <https://www.w3.org/TR/SVG11/intro.html#TermGraphicsElement>
/// Note `reuse` is not a standard SVG element, but is used here in similar
/// contexts to the `use` element.
fn is_graphics_element(el: &SvgElement) -> bool {
    matches!(
        el.name(),
        "circle"
                | "ellipse"
                | "image"
                | "line"
                | "path"
                | "polygon"
                | "polyline"
                | "rect"
                | "text"
                | "use"
                // Following are non-standard.
                | "reuse"
    )
}

/// See <https://www.w3.org/TR/SVG11/intro.html#TermContainerElement>
/// Note `specs` is not a standard SVG element, but is used here in similar
/// contexts to the `defs` element.
#[allow(dead_code)]
fn is_container_element(el: &SvgElement) -> bool {
    matches!(
        el.name(),
        "a" | "defs"
                | "glyph"
                | "g"
                | "marker"
                | "mask"
                | "missing-glyph"
                | "pattern"
                | "svg"
                | "switch"
                | "symbol"
                // Following not listed as a 'container element', but acts like it
                | "clipPath"
                // Following are non-standard.
                | "specs"
    )
}
