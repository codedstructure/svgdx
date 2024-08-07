use crate::context::TransformerContext;
use crate::element::SvgElement;
use crate::events::{EventList, SvgEvent};
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
    mut event_element: SvgElement,
    idx_output: &mut BTreeMap<OrderIndex, EventList>,
) -> Result<SvgElement> {
    let elref = event_element
        .pop_attr("href")
        .context("reuse element should have an href attribute")?;
    let referenced_element = context
        .get_original_element(
            elref
                .strip_prefix('#')
                .context("href value should begin with '#'")?,
        )
        .context("unknown reference")?
        .to_owned();
    let mut instance_element = referenced_element.clone();

    // the referenced element will have an `id` attribute (which it was
    // referenced by) but the new instance should not have this to avoid
    // multiple elements with the same id. We remove it here and re-add as
    // a class.
    // However we *do* want the instance element to inherit any `id` which
    // was on the `reuse` element.
    let ref_id = instance_element
        .pop_attr("id")
        .context("referenced element should have id")?;

    if referenced_element.name == "g" {
        if let Some((start, end)) = referenced_element.event_range {
            // opening g element is not included in the processed inner events to avoid
            // infinite recursion...
            let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);
            // ...but we do want to include it for attribute-variable lookups, so push the
            // referenced element onto the element stack (just while we run process_events)
            context.push_element(referenced_element.to_ell());
            let g_events = process_events(inner_events, context)?;
            context.pop_element();

            let mut group_element = SvgElement::new("g", &[]);
            group_element.set_indent(event_element.indent);
            group_element.set_src_line(event_element.src_line);
            group_element.add_classes(&event_element.classes);
            group_element.add_class(&ref_id);
            if let Some(inst_id) = event_element.pop_attr("id") {
                group_element.set_attr("id", &inst_id);
                context.update_element(&group_element);
            }
            let group_open = EventList::from(SvgEvent::Start(group_element));
            let group_close = EventList::from(SvgEvent::End("g".to_string()));
            idx_output.insert(event_element.order_index.with_index(0), group_open);
            idx_output.insert(event_element.order_index.with_index(1), g_events);
            idx_output.insert(event_element.order_index.with_index(2), group_close);

            return Ok(SvgElement::new("phantom", &[]));
        }
    }

    if let Some(inst_id) = event_element.pop_attr("id") {
        instance_element.set_attr("id", &inst_id);
        context.update_element(&event_element);
    }
    // the instanced element should have the same indent as the original
    // `reuse` element, as well as inherit `style` and `class` values.
    instance_element.set_indent(event_element.indent);
    instance_element.set_src_line(event_element.src_line);
    if let Some(inst_style) = event_element.pop_attr("style") {
        instance_element.set_attr("style", &inst_style);
    }
    instance_element.add_classes(&event_element.classes);
    instance_element.add_class(&ref_id);
    Ok(instance_element)
}
