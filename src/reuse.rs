use crate::context::TransformerContext;
use crate::element::SvgElement;
use crate::events::{EventList, SvgEvent};
use crate::expression::eval_attr;
use crate::transform::{process_events, ElementLike};

use anyhow::{Context, Result};
use itertools::Itertools;

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

    fn generate_events(&self, context: &mut TransformerContext) -> Result<EventList> {
        handle_reuse_element(context, self.0.clone())
    }
}

pub fn handle_reuse_element(
    context: &mut TransformerContext,
    event_element: SvgElement,
) -> Result<EventList> {
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
        .with_context(|| format!("unknown reference '{}'", elref))?
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

        // Resulting order: instance transform, reuse transform, x/y transform
        let inst_xfrm = instance_element.get_attr("transform");
        let reuse_xfrm = event_element.get_attr("transform");
        let xfrm: Vec<_> = [inst_xfrm, reuse_xfrm, xy_xfrm]
            .into_iter()
            .flatten()
            .collect();

        if !xfrm.is_empty() {
            let xfrm = xfrm.iter().join(" ");
            instance_element.set_attr("transform", &xfrm);
        }
    }

    // reuse of a symbol element wraps the resulting content in a new <g> element
    if instance_element.name == "symbol" {
        instance_element = SvgElement::new("g", &[]).with_attrs_from(&instance_element);
    }

    if !instance_element.is_empty_element() {
        if let Some((start, end)) = instance_element.event_range {
            let tag_name = instance_element.name.clone();

            let mut new_events = EventList::new();
            new_events.push(SvgEvent::Start(instance_element));
            new_events.extend(&EventList::from(context.events.clone()).slice(start + 1, end));
            new_events.push(SvgEvent::End(tag_name));
            for (idx, ev) in new_events.iter_mut().enumerate() {
                // Hack: the index of each event is used in by process_seq to update
                // the resulting BTreeMap, so needs to be different/increasing.
                // TODO: warning if existing entry is overwritten, and a better way of
                // doing this.
                ev.index = idx;
            }
            context.push_element(event_element.to_ell());
            let g_events = process_events(new_events, context);
            context.pop_element();

            return g_events;
        }
    }

    instance_element.to_ell().borrow().generate_events(context)
}
