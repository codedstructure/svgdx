use super::SvgElement;
use crate::context::TransformerContext;
use crate::errors::{Result, SvgdxError};
use crate::events::OutputList;
use crate::expression::{eval_attr, eval_condition};
use crate::geometry::BoundingBox;
use crate::transform::{process_events, EventGen};

#[derive(Debug, Clone)]
pub struct DefaultsElement(pub SvgElement);

impl EventGen for DefaultsElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        for ev in self.0.inner_events(context).unwrap_or_default() {
            // we only care about Element-generating (i.e. start/empty) events
            if let Ok(el) = SvgElement::try_from(ev.clone()) {
                context.set_element_default(&el);
            }
        }
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
pub struct ConfigElement(pub SvgElement);

impl EventGen for ConfigElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut new_config = context.config.clone();
        for (key, value) in &self.0.attrs {
            match key.as_str() {
                "scale" => new_config.scale = value.parse()?,
                "debug" => new_config.debug = value.parse()?,
                "add-auto-styles" => new_config.add_auto_styles = value.parse()?,
                "use-local-styles" => new_config.use_local_styles = value.parse()?,
                "border" => new_config.border = value.parse()?,
                "background" => new_config.background.clone_from(value),
                "loop-limit" => new_config.loop_limit = value.parse()?,
                "var-limit" => new_config.var_limit = value.parse()?,
                "depth-limit" => new_config.depth_limit = value.parse()?,
                "font-size" => new_config.font_size = value.parse()?,
                "font-family" => new_config.font_family.clone_from(value),
                "seed" => new_config.seed = value.parse()?,
                "theme" => new_config.theme = value.parse()?,
                "svg-style" => new_config.svg_style = Some(value.clone()),
                _ => {
                    return Err(SvgdxError::InvalidData(format!(
                        "Unknown config setting {key}"
                    )))
                }
            }
        }
        context.set_config(new_config);
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
pub struct SpecsElement(pub SvgElement);

impl EventGen for SpecsElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        if context.in_specs {
            return Err(SvgdxError::DocumentError(
                "Nested <specs> elements are not allowed".to_string(),
            ));
        }
        if let Some(inner_events) = self.0.inner_events(context) {
            context.in_specs = true;
            process_events(inner_events, context)?;
            context.in_specs = false;
        }
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
pub struct VarElement(pub SvgElement);

impl EventGen for VarElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        // variables are updated 'in parallel' rather than one-by-one,
        // allowing e.g. swap in a single `<var>` element:
        // `<var a="$b" b="$a" />`
        let mut new_vars = Vec::new();
        for (key, value) in self.0.attrs.clone() {
            // Note comments in `var` elements are permitted (and encouraged!)
            // in the input, but not propagated to the output.
            if key != "_" && key != "__" {
                let value = eval_attr(&value, context)?;
                // Detect / prevent uncontrolled expansion of variable values
                if value.len() > context.config.var_limit as usize {
                    return Err(SvgdxError::VarLimitError(
                        key.clone(),
                        value.len(),
                        context.config.var_limit,
                    ));
                }
                new_vars.push((key, value));
            }
        }
        for (k, v) in new_vars.into_iter() {
            context.set_var(&k, &v);
        }
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
pub struct IfElement(pub SvgElement);

impl EventGen for IfElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let test = self
            .0
            .get_attr("test")
            .ok_or_else(|| SvgdxError::MissingAttribute("test".to_owned()))?;
        if let Some(inner_events) = self.0.inner_events(context) {
            if eval_condition(&test, context)? {
                // opening if element is not included in the processed inner events to avoid
                // infinite recursion...
                return process_events(inner_events.clone(), context);
            }
        }

        Ok((OutputList::new(), None))
    }
}
