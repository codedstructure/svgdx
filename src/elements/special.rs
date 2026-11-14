use super::SvgElement;
use crate::context::TransformerContext;
use crate::document::OutputList;
use crate::errors::{Error, Result};
use crate::expr::eval_attr;
use crate::style::{ThemeType, extract_themes};
use crate::transform::{EventGen, process_events};
use crate::types::{parse_bool, parse_float, parse_int};

#[derive(Debug, Clone)]
pub struct DefaultsElement<'a>(pub &'a SvgElement);

impl EventGen for DefaultsElement<'_> {
    fn generate_events(&self, context: &mut TransformerContext) -> Result<OutputList> {
        // both the `defaults` element itself and any inner elements
        // are used to set default attr values.
        context.set_element_default(self.0);
        for ev in self.0.inner_events(context).unwrap_or_default() {
            // we only care about Element-generating (i.e. start/empty) events
            if let Ok(el) = SvgElement::try_from(ev.clone()) {
                context.set_element_default(&el);
            }
        }
        Ok(OutputList::new())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigElement<'a>(pub &'a SvgElement);

impl EventGen for ConfigElement<'_> {
    fn generate_events(&self, context: &mut TransformerContext) -> Result<OutputList> {
        let mut new_config = context.config.clone();
        for (key, value) in self.0.get_attrs() {
            match key.as_str() {
                "scale" => new_config.scale = parse_float(value)?,
                "debug" => new_config.debug = parse_bool(value)?,
                "auto-style-mode" => new_config.auto_style_mode = value.parse()?,
                "border" => new_config.border = parse_int(value)?,
                "background" => new_config.background = value,
                "loop-limit" => new_config.loop_limit = parse_int(value)?,
                "var-limit" => new_config.var_limit = parse_int(value)?,
                "depth-limit" => new_config.depth_limit = parse_int(value)?,
                "path-repeat-limit" => new_config.path_repeat_limit = parse_int(value)?,
                "font-size" => new_config.font_size = parse_float(value)?,
                "font-family" => new_config.font_family = value,
                "seed" => new_config.seed = parse_int(value)?,
                "theme" => {
                    new_config.theme = value
                        .parse()
                        .unwrap_or_else(|_| ThemeType::Custom(value.clone()))
                }
                "svg-style" => new_config.svg_style = Some(value.clone()),
                "error-mode" => new_config.error_mode = value.parse()?,
                _ => return Err(Error::InvalidAttr(key)),
            }
        }
        context.set_config(new_config)?;
        Ok(OutputList::new())
    }
}

#[derive(Debug, Clone)]
pub struct SpecsElement<'a>(pub &'a SvgElement);

/// Element names reserved by svgdx that cannot be used as custom element names.
const SPECS_RESERVED_ELEMENT_NAMES: &[&str] = &[
    "loop",
    "config",
    "reuse",
    "specs",
    "var",
    "varDefault",
    "varTry",
    "varTryDefault",
    "if",
    "defaults",
    "for",
    "themes",
];

impl EventGen for SpecsElement<'_> {
    fn generate_events(&self, context: &mut TransformerContext) -> Result<OutputList> {
        // support '<specs element="thing">...</specs>' as a shortcut for defining
        // reuse triggered by an element name rather than an href to an id.
        if let Some(element_name) = self.0.get_attr("element") {
            let element_name = eval_attr(element_name, context)?;
            if SPECS_RESERVED_ELEMENT_NAMES.contains(&element_name.as_str()) {
                return Err(Error::InvalidValue(
                    "reserved specs 'element' value".into(),
                    element_name.clone(),
                ));
            }
            // Create a synthetic <symbol id="element_name"> wrapping the inner events,
            // and register it for reuse lookup.
            let symbol = SvgElement::new("symbol", &[("id".into(), element_name.clone())])
                .with_attrs_from(self.0)
                .without_attr("element");
            context.register_named_spec(element_name, symbol);
        }
        if let Some(inner_events) = self.0.inner_events(context) {
            // Process inner events, but throw away non-error result.
            process_events(inner_events, context)?;
        }
        Ok(OutputList::new())
    }
}

fn process_var_attrs<F>(
    element: &SvgElement,
    context: &mut TransformerContext,
    ignore_fail: bool,
    set_fn: F,
) -> Result<()>
where
    F: Fn(&mut TransformerContext, &str, &str),
{
    // variables are updated 'in parallel' rather than one-by-one,
    // allowing e.g. swap in a single `<var>` element:
    // `<var a="$b" b="$a" />`
    let mut new_vars = Vec::new();
    for (key, value) in element.get_attrs() {
        // Note comments in `var` elements are permitted (and encouraged!)
        // in the input, but not propagated to the output.
        // Errors from eval_attr prevent creation of the variable,
        // but are otherwise ignored.
        if key == "_" || key == "__" {
            continue;
        }
        match eval_attr(&value, context) {
            Ok(value) => {
                // Detect / prevent uncontrolled expansion of variable values
                if value.len() > context.config.var_limit as usize {
                    return Err(Error::VarLimit(
                        key.clone(),
                        value.len(),
                        context.config.var_limit,
                    ));
                }
                new_vars.push((key, value));
            }
            Err(e) => {
                if !ignore_fail {
                    return Err(e);
                }
            }
        }
    }
    for (k, v) in new_vars.into_iter() {
        set_fn(context, &k, &v);
    }
    Ok(())
}

/// A `<themes>` element registers the `<theme>` elements it contains
/// for later selection, generating no output.
#[derive(Debug, Clone)]
pub struct ThemesElement<'a>(pub &'a SvgElement);

impl EventGen for ThemesElement<'_> {
    fn generate_events(&self, context: &mut TransformerContext) -> Result<OutputList> {
        if let Some(inner_events) = self.0.inner_events(context) {
            for theme in extract_themes(inner_events.iter(), 0) {
                context.register_theme(theme);
            }
        }
        Ok(OutputList::new())
    }
}

fn var_element_kind(name: &str) -> (bool, bool) {
    // (ignore_fail, is_default)
    match name {
        "var" => (false, false),
        "varTry" => (true, false),
        "varDefault" => (false, true),
        "varTryDefault" => (true, true),
        _ => (false, false),
    }
}

/// Evaluate a `<var>` element generated from a theme, setting variables in
/// the global scope regardless of where the theme was selected from.
pub fn apply_theme_var(element: &SvgElement, context: &mut TransformerContext) -> Result<()> {
    process_var_attrs(element, context, false, |ctx, k, v| {
        ctx.set_global_var(k, v);
    })
}

#[derive(Debug, Clone)]
pub struct VarElement<'a>(pub &'a SvgElement);

impl EventGen for VarElement<'_> {
    fn generate_events(&self, context: &mut TransformerContext) -> Result<OutputList> {
        let (ignore_fail, is_default) = var_element_kind(self.0.name());
        process_var_attrs(self.0, context, ignore_fail, |ctx, k, v| {
            if is_default {
                ctx.set_var_default(k, v);
            } else {
                ctx.set_var(k, v)
            }
        })?;
        Ok(OutputList::new())
    }
}
