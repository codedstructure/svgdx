use crate::context::{ElementMap, TransformerContext};
use crate::element::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::events::{InputEvent, InputList, OutputEvent, OutputList};
use crate::position::{BoundingBox, Position};
use crate::transform::{process_events, EventGen};
use crate::types::ElRef;

#[derive(Debug, Clone)]
pub struct ReuseElement(pub SvgElement);

impl EventGen for ReuseElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut reuse_element = self.0.clone();

        // first resolve any attributes on the immediate reuse element;
        // we later resolve those on the target element in the context
        // of any vars set by this.
        reuse_element.eval_attributes(context)?;

        context.push_element(&reuse_element);
        let elref = reuse_element
            .get_attr("href")
            .ok_or_else(|| SvgdxError::MissingAttribute("href".to_owned()))?;
        let elref: ElRef = elref.parse()?;
        // Take a copy of the referenced element as starting point for our new instance
        let mut instance_element = context
            .get_original_element(&elref)
            .ok_or_else(|| SvgdxError::ReferenceError(elref.clone()))?
            .clone();
        instance_element.expand_compound_size();
        instance_element.eval_attributes(context).inspect_err(|_| {
            context.pop_element();
        })?;
        let instance_size = instance_element.size(context)?;

        // Override 'default' attr values in the target
        for (attr, value) in reuse_element.get_attrs() {
            match attr.as_str() {
                "href" | "id" | "x" | "y" => continue,
                "transform" => {
                    // append to any existing transform
                    let mut xfrm = value.clone();
                    if let Some(inst_xfrm) = instance_element.get_attr("transform") {
                        xfrm = format!("{} {}", inst_xfrm, xfrm);
                    }
                    instance_element.set_attr("transform", &xfrm);
                }
                _ => {
                    // this is the _opposite_ of set_default_attr(); it allows
                    // the target element to provide defaults, but have them
                    // overridden by the reuse element.
                    if instance_element.has_attr(&attr) {
                        instance_element.set_attr(&attr, &value);
                    }
                }
            }
        }

        // if referenced by an ElRef::Id (rather than Prev), will have an `id`
        // attribute (which it was referenced by) but the new instance should
        // not have this to avoid multiple elements with the same id.
        // We remove it here and re-add as a class.
        // However we *do* want the instance element to inherit any `id` which
        // was on the `reuse` element.
        let ref_id = instance_element.pop_attr("id");
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
        if let Some(ref_id) = ref_id {
            instance_element.add_class(&ref_id);
        }

        // reuse of a symbol element wraps the resulting content in a new <g> element
        if instance_element.name == "symbol" {
            instance_element = SvgElement::new("g", &[]).with_attrs_from(&instance_element);
        }

        // TODO: This isn't ideal. resolve_position() is needed to handle
        // relpos positioning (`xy="#a|h"` etc), but the Position-based
        // stuff fully handles other positioning. Should be unified.
        reuse_element.resolve_position(context)?;

        let inst_el = context
            .get_element(&elref)
            .ok_or_else(|| SvgdxError::ReferenceError(elref.clone()))?;
        let mut pos = Position::from(&reuse_element);
        if let Some(bb) = inst_el.content_bbox {
            pos.update_size(&bb.size());
        } else if let Some(sz) = instance_size {
            pos.update_size(&sz);
        }
        pos.update_shape(&instance_element.name);
        pos.set_position_attrs(&mut instance_element);

        let res = if let (false, Some((start, end))) = (
            instance_element.is_empty_element(),
            instance_element.event_range,
        ) {
            // we've changed the initial (and possibly closing) tag of the instance element,
            // so we create a new list including that and process it.
            let mut new_events = InputList::new();
            let tag_name = instance_element.name.clone();
            let mut start_ev = InputEvent::from(OutputEvent::Start(instance_element));
            start_ev.index = start;
            start_ev.alt_idx = Some(end);
            new_events.push(start_ev);
            new_events.extend(&InputList::from(&context.events[start + 1..end]));
            let mut end_ev = InputEvent::from(OutputEvent::End(tag_name));
            end_ev.index = end;
            end_ev.alt_idx = Some(start);
            new_events.push(end_ev);
            process_events(new_events, context)
        } else {
            instance_element.generate_events(context)
        };
        context.pop_element();
        res
    }
}
