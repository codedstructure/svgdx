use super::SvgElement;
use crate::context::TransformerContext;
use crate::errors::Result;
use crate::events::{OutputEvent, OutputList};
use crate::geometry::BoundingBox;
use crate::transform::{process_events_with_index, EventGen};

/// Container will be used for many elements which contain other elements,
/// but have no independent behaviour, such as defs, linearGradient, etc.
#[derive(Debug, Clone)]
pub struct Container(pub SvgElement);

impl EventGen for Container {
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
            if let (true, Some(text)) = (is_graphics_element(&self.0), &inner_text) {
                let mut el = self.0.clone();
                el.set_attr("text", text);
                if let Some((start, _end)) = self.0.event_range {
                    el.event_range = Some((start, start)); // emulate an Empty element
                }
                el.generate_events(context)
            } else {
                let mut new_el = self.0.clone();
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
                events.push(OutputEvent::Start(new_el.clone()));
                let (evlist, mut bbox) = if inner_text.is_some() {
                    // inner_text implies no processable events; use as-is
                    (inner_events.into(), None)
                } else {
                    let oi_base = Some(new_el.order_index.clone());
                    process_events_with_index(inner_events, context, oi_base)?
                };
                events.extend(&evlist);
                events.push(OutputEvent::End(self.0.name().to_owned()));

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
pub struct GroupElement(pub SvgElement);

impl EventGen for GroupElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        // since we synthesize the opening element event here, we need to
        // do any required transformations on the <g> itself here.
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context)?;

        // push variables onto the stack
        context.push_element(&self.0);

        let (inner, content_bb) = if let Some(inner_events) = self.0.inner_events(context) {
            // get the inner events / bbox first, as some outer element attrs
            // (e.g. `transform` via rotate) may depend on the bbox.
            let oi_base = Some(new_el.order_index.clone());
            process_events_with_index(inner_events, context, oi_base).inspect_err(|_| {
                context.pop_element();
            })?
        } else {
            (OutputList::new(), None)
        };

        // pop variables off the stack
        context.pop_element();

        // Need bbox to provide center of rotation
        new_el.content_bbox = content_bb;
        new_el.handle_rotation()?;

        let mut events = OutputList::new();
        if self.0.is_empty_element() {
            events.push(OutputEvent::Empty(new_el.clone()));
        } else {
            let el_name = new_el.name().to_owned();
            events.push(OutputEvent::Start(new_el.clone()));
            events.extend(&inner);
            events.push(OutputEvent::End(el_name));
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
                // Following are non-standard.
                | "specs"
    )
}
