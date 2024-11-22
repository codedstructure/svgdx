use crate::context::TransformerContext;
use crate::element::SvgElement;
use crate::events::{EventList, InputEvent, SvgEvent};
use crate::expression::eval_attr;
use crate::position::BoundingBox;
use crate::transform::{process_events, ElementLike};

use anyhow::{Context, Result};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct ReuseElement(pub SvgElement);

impl ElementLike for ReuseElement {
    fn get_element(&self) -> Option<SvgElement> {
        Some(self.0.clone())
    }

    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        let mut reuse_element = self.0.clone();

        context.push_element(reuse_element.to_ell());
        reuse_element.eval_attributes(context);
        let elref = reuse_element
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
        if let Some(inst_id) = reuse_element.get_attr("id") {
            instance_element.set_attr("id", &inst_id);
            context.update_element(&reuse_element);
        }
        // the instanced element should have the same indent as the original
        // `reuse` element, as well as inherit `style` and `class` values.
        instance_element.set_indent(reuse_element.indent);
        instance_element.set_src_line(reuse_element.src_line);
        if let Some(inst_style) = reuse_element.get_attr("style") {
            instance_element.set_attr("style", &inst_style);
        }
        instance_element.add_classes(&reuse_element.classes);
        instance_element.add_class(&ref_id);

        // Emulate (a bit) the `<use>` element - in particular `transform` is passed through
        // and any x/y attrs become a new (final) entry in the `transform`.
        // TODO: ensure transform() is considered by bbox() / positioning.
        {
            let reuse_x = reuse_element.get_attr("x");
            let reuse_y = reuse_element.get_attr("y");
            let xy_xfrm = if reuse_x.is_some() || reuse_y.is_some() {
                let reuse_x = eval_attr(&reuse_x.unwrap_or("0".to_string()), context);
                let reuse_y = eval_attr(&reuse_y.unwrap_or("0".to_string()), context);
                Some(format!("translate({reuse_x}, {reuse_y})"))
            } else {
                None
            };

            // Resulting order: instance transform, reuse transform, x/y transform
            let inst_xfrm = instance_element.get_attr("transform");
            let reuse_xfrm = reuse_element.get_attr("transform");
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

        let res = if let (false, Some((start, end))) = (
            instance_element.is_empty_element(),
            instance_element.event_range,
        ) {
            // we've changed the initial (and possibly closing) tag of the instance element,
            // so we create a new list including that and process it.
            let mut new_events = EventList::new();
            let tag_name = instance_element.name.clone();
            let mut start_ev = InputEvent::from(SvgEvent::Start(instance_element));
            start_ev.index = start;
            start_ev.alt_idx = Some(end);
            new_events.push(start_ev);
            new_events.extend(&EventList::from(context.events.clone()).slice(start + 1, end));
            let mut end_ev = InputEvent::from(SvgEvent::End(tag_name));
            end_ev.index = end;
            end_ev.alt_idx = Some(start);
            new_events.push(end_ev);
            process_events(new_events, context)
        } else {
            instance_element.to_ell().borrow().generate_events(context)
        };
        // context.pop_element(); //ush_element(reuse_element.to_ell());
        context.push_element(reuse_element.to_ell());
        res
    }
}
