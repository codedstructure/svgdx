use crate::context::TransformerContext;
use crate::element::SvgElement;
use crate::events::{EventList, SvgEvent};
use crate::expression::eval_attr;
use crate::transform::{process_events, ElementLike};
use crate::types::OrderIndex;
use std::collections::BTreeMap;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ReuseElement(pub SvgElement);

impl ElementLike for ReuseElement {
    fn handle_element_start(
        &mut self,
        element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        // Even though `<reuse>`` is (typically) an Empty element, it acts as a
        // container element around the referenced element for variable lookup.
        context.push_element(element.to_ell());
        Ok(())
    }

    fn handle_element_end(
        &mut self,
        _element: &mut SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        context.pop_element();
        Ok(())
    }

    fn get_element(&self) -> Option<SvgElement> {
        Some(self.0.clone())
    }
}

// TODO: this should really be called from ReuseElement::generate_events(),
// not special-cased from process_seq().
pub fn handle_reuse_element(
    context: &mut TransformerContext,
    event_element: SvgElement,
    idx_output: &mut BTreeMap<OrderIndex, EventList>,
) -> Result<SvgElement> {
    let elref = event_element
        .get_attr("href")
        .context("reuse element should have an href attribute")?;
    // Take a copy of the referenced element as starting point for our new instance
    let mut instance_element = context
        .get_original_element(
            elref
                .strip_prefix('#')
                .context("href value should begin with '#'")?,
        )
        .context("unknown reference")?
        .clone();

    // the referenced element will have an `id` attribute (which it was
    // referenced by) but the new instance should not have this to avoid
    // multiple elements with the same id. We remove it here and re-add as
    // a class.
    // However we *do* want the instance element to inherit any `id` which
    // was on the `reuse` element.
    let ref_id = instance_element
        .pop_attr("id")
        .context("referenced element should have id")?;
    if let Some(inst_id) = event_element.get_attr("id") {
        instance_element.set_attr("id", &inst_id);
        context.update_element(&event_element);
    }
    // the instanced element should have the same indent as the original
    // `reuse` element, as well as inherit `style` and `class` values.
    instance_element.set_indent(event_element.indent);
    instance_element.set_src_line(event_element.src_line);
    if let Some(inst_style) = event_element.get_attr("style") {
        instance_element.set_attr("style", &inst_style);
    }
    instance_element.add_classes(&event_element.classes);
    instance_element.add_class(&ref_id);

    // TODO: or "symbol", needs testing.
    if instance_element.name == "g" {
        if let Some((start, end)) = instance_element.event_range {
            // opening g element is not included in the processed inner events to avoid
            // infinite recursion...
            let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);
            // ...but we do want to include it for attribute-variable lookups, so push the
            // referenced element onto the element stack (just while we run process_events)
            context.push_element(event_element.to_ell());
            context.push_element(instance_element.to_ell());
            let g_events = process_events(inner_events, context)?;
            context.pop_element();
            context.pop_element();

            // Emulate (a bit) the `<use>` element - in particular `transform` is passed through
            // and any x/y attrs become a new (final) entry in the `transform`.

            // TODO: ensure transform() is considered by bbox() / positioning.
            {
                let reuse_x = event_element.get_attr("x");
                let reuse_y = event_element.get_attr("y");
                let xy_xfrm = if reuse_x.is_some() || reuse_y.is_some() {
                    let reuse_x = eval_attr(&reuse_x.unwrap_or("0".to_string()), context);
                    let reuse_y = eval_attr(&reuse_y.unwrap_or("0".to_string()), context);
                    Some(format!("translate({reuse_x}, {reuse_y})"))
                } else {
                    None
                };

                let orig_xfrm = event_element.get_attr("transform");
                let xfrm = if let (Some(xfrm), Some(xy_xfrm)) = (&orig_xfrm, &xy_xfrm) {
                    let xfrm = eval_attr(xfrm, context);
                    Some(format!("{xfrm} {xy_xfrm}"))
                } else if let Some(xfrm) = orig_xfrm {
                    Some(xfrm)
                } else {
                    xy_xfrm
                };
                if let Some(xfrm) = xfrm {
                    instance_element.set_attr("transform", &xfrm);
                }
            }

            let group_open = EventList::from(SvgEvent::Start(instance_element));
            let group_close = EventList::from(SvgEvent::End("g".to_string()));
            idx_output.insert(event_element.order_index.with_index(0), group_open);
            idx_output.insert(event_element.order_index.with_index(1), g_events);
            idx_output.insert(event_element.order_index.with_index(2), group_close);

            return Ok(SvgElement::new("phantom", &[]));
        }
    }

    Ok(instance_element)
}
