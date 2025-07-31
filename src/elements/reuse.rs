use super::SvgElement;
use crate::context::{ElementMap, TransformerContext};
use crate::errors::{Result, SvgdxError};
use crate::events::{InputEvent, InputList, OutputEvent, OutputList};
use crate::geometry::{BoundingBox, Position};
use crate::transform::{process_events, EventGen};
use crate::types::{fstr, strp, ElRef};

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
            .ok_or_else(|| SvgdxError::MissingAttribute("href".to_owned()))
            .inspect_err(|_| {
                context.pop_element();
            })?;
        let elref: ElRef = elref.parse().inspect_err(|_| {
            context.pop_element();
        })?;
        // Take a copy of the referenced element as starting point for our new instance
        let mut instance_element = context
            .get_original_element(&elref)
            .cloned()
            .ok_or_else(|| SvgdxError::ReferenceError(elref.clone()))
            .inspect_err(|_| {
                context.pop_element();
            })?;
        // TODO: this replicates some of the logic from OtherElement::generate_events()
        // but maybe should just generate the inner events first rather than later in
        // this function? The current approach assumes that we can't generate events
        // until we know position, which is true for 'simple' elements, but not for
        // containers, where we use transform on the outer element. Potentially we could
        // wrap *every* rendered reuse instance in a <g> element with a transform...
        // (not keen...)
        instance_element
            .resolve_position(context)
            .inspect_err(|_| {
                context.pop_element();
            })?;
        instance_element.transmute(context).inspect_err(|_| {
            context.pop_element();
        })?;
        let instance_size = instance_element.size(context)?;

        // Override 'default' attr values in the target
        for (attr, value) in reuse_element.get_attrs() {
            match attr.as_str() {
                "href" | "id" | "x" | "y" => continue,
                "rotate" | "text-rotate" => {
                    // any existing rotation is built on by the reuse element
                    if let Some(inst_rot) = instance_element.get_attr(&attr) {
                        let inst_rot = strp(inst_rot)?;
                        let rot = strp(&value)?;
                        instance_element.set_attr(&attr, &fstr(inst_rot + rot));
                    } else {
                        instance_element.set_attr(&attr, &value);
                    }
                }
                "transform" => {
                    // append to any existing transform
                    let mut xfrm = value.clone();
                    if let Some(inst_xfrm) = instance_element.get_attr("transform") {
                        xfrm = format!("{inst_xfrm} {xfrm}");
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
            instance_element.set_attr("id", inst_id);
            context.update_element(&reuse_element);
        }
        // the instanced element should have the same indent as the original
        // `reuse` element, as well as inherit `style` and `class` values.
        instance_element.set_indent(reuse_element.indent);
        instance_element.set_src_line(reuse_element.src_line);
        if let Some(inst_style) = reuse_element.get_attr("style") {
            instance_element.set_attr("style", inst_style);
        }
        instance_element.add_classes_from(&reuse_element);
        if let Some(ref_id) = ref_id {
            instance_element.add_class(&ref_id);
        }

        // reuse of a symbol element wraps the resulting content in a new <g> element
        if instance_element.name() == "symbol" {
            instance_element = SvgElement::new("g", &[]).with_attrs_from(&instance_element);
        }

        let mut pos = match reuse_element
            .extract_relpos()
            .map(|relpos| reuse_element.pos_from_dirspec(&relpos, context))
        {
            Some(Ok(Some(relpos))) => relpos,
            _ => {
                reuse_element.resolve_position(context)?;
                Position::try_from(&reuse_element)?
            }
        };

        let inst_el = context
            .get_element(&elref)
            .ok_or_else(|| SvgdxError::ReferenceError(elref.clone()))?;
        if let Some(bb) = inst_el.content_bbox {
            pos.update_size(&bb.size());
        } else if let Some(sz) = instance_size {
            pos.update_size(&sz);
        }
        pos.update_shape(instance_element.name());
        pos.set_position_attrs(&mut instance_element);

        let res = if let (false, Some((start, end))) = (
            instance_element.is_empty_element(),
            instance_element.event_range,
        ) {
            // we've changed the initial (and possibly closing) tag of the instance element,
            // so we create a new list including that and process it.
            let mut new_events = InputList::new();
            let tag_name = instance_element.name().to_owned();
            let outer_events = instance_element.all_events(context);
            let mut start_ev = InputEvent::from(OutputEvent::Start(instance_element));
            start_ev.index = start;
            start_ev.alt_idx = Some(end);
            new_events.push(start_ev);
            new_events.extend(&outer_events);
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
