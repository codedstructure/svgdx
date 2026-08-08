use super::SvgElement;
use crate::context::TransformerContext;
use crate::document::OutputList;
use crate::errors::{Error, Result};
use crate::geometry::BoundingBox;
use crate::transform::EventGen;
use crate::types::{ElRef, strp};

#[derive(Debug, Clone)]
pub struct ReuseElement<'a>(pub &'a SvgElement);

impl EventGen for ReuseElement<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut reuse_element = self.0.clone();

        // first resolve any attributes on the immediate reuse element;
        // we later resolve those on the target element in the context
        // of any vars set by this.
        reuse_element.eval_attributes(context)?;
        let scope_element = reuse_element.clone();

        context.with_element_scope(&scope_element, |context| {
            let elref = reuse_element
                .get_attr("href")
                .ok_or_else(|| Error::MissingAttr("href".to_owned()))?;
            let elref: ElRef = elref.parse()?;
            // Take a copy of the referenced element as starting point for our new instance
            let mut instance_element = context
                .get_original_element(&elref)
                .cloned()
                .ok_or_else(|| Error::Reference(elref.clone()))?;

            instance_element.set_order_index(&reuse_element.order_index);
            instance_element.resolve_position(context)?;
            let should_include = instance_element.transmute(context)?;
            if !should_include {
                // Element should be skipped (e.g. overlapping connectors)
                return Ok((OutputList::new(), None));
            }

            // Override 'default' attr values in the target
            for (attr, value) in reuse_element.get_attrs() {
                match attr.as_str() {
                    "href" | "id" => continue,
                    "rotate" | "text-rotate" => {
                        // any existing rotation is built on by the reuse element
                        if let Some(inst_rot) = instance_element.get_num_attr(&attr)? {
                            let rot = strp(&value)?;
                            instance_element.set_num_attr(&attr, inst_rot + rot);
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
                    "xy-loc" | "xy" | "cxy" | "xy1" | "xy2" | "x" | "y" | "cx" | "cy" | "x1"
                    | "y1" | "x2" | "y2" | "dx" | "dy" | "dxy" => {
                        instance_element.set_attr(&attr, &value);
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

            // if referenced by an ElRef::Id (rather than Prev/Next), will have an `id`
            // attribute (which it was referenced by) but the new instance should
            // not have this to avoid multiple elements with the same id.
            // We remove it here and re-add as a class.
            // However we *do* want the instance element to inherit any `id` which
            // was on the `reuse` element.
            let ref_id = instance_element.pop_attr("id");
            if let Some(inst_id) = reuse_element.get_attr("id") {
                instance_element.set_attr("id", inst_id);
            }
            // the instanced element should have the same indent as the original
            // `reuse` element, as well as inherit `style` and `class` values.
            instance_element.set_indent(reuse_element.indent);
            instance_element.set_src_line(reuse_element.src_line);
            instance_element.add_styles_from(&reuse_element);
            instance_element.add_classes_from(&reuse_element);
            instance_element.set_order_index(&reuse_element.order_index);
            if let Some(ref_id) = ref_id {
                instance_element.add_class(&ref_id);
            }

            // reuse of a symbol element wraps the resulting content in a new <g> element
            if instance_element.name() == "symbol" {
                instance_element = SvgElement::new("g", &[]).with_attrs_from(&instance_element);
            }

            instance_element.generate_events(context)
        })
    }
}
