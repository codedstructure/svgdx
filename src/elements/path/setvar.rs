use std::cell::RefCell;
use std::collections::HashMap;

use rand_pcg::Pcg32;

use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::Result;
use crate::context::ContextView;
use crate::elements::SvgElement;
use crate::geometry::BoundingBox;
use crate::types::{VarName, fstr};

pub struct SetVar {
    name: String,
    value: String,
}

impl SetVar {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, ctx: &impl ContextView) -> Result<Self> {
        let name = tokens.read_identifier()?;
        tokens.skip_whitespace();
        let value = tokens.read_number(ctx)?;
        Ok(Self {
            name: name.parse::<VarName>()?.to_string(),
            value: fstr(value),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

pub(super) struct PathEvalContext<'a, C: ContextView> {
    pub base: &'a C,
    pub vars: &'a HashMap<String, String>,
}

impl<C: ContextView> crate::context::ElementMap for PathEvalContext<'_, C> {
    fn set_current_element(&mut self, _el: &SvgElement) {}

    fn get_element(&self, elref: &crate::types::ElRef) -> Option<&SvgElement> {
        self.base.get_element(elref)
    }

    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>> {
        self.base.get_element_bbox(el)
    }

    fn get_element_size(&self, el: &SvgElement) -> Result<Option<crate::geometry::Size>> {
        self.base.get_element_size(el)
    }

    fn get_target_element(&self, el: &SvgElement) -> Result<SvgElement> {
        self.base.get_target_element(el)
    }
}

impl<C: ContextView> crate::context::VariableMap for PathEvalContext<'_, C> {
    fn get_var(&self, name: &str) -> Option<String> {
        self.vars
            .get(name)
            .cloned()
            .or_else(|| self.base.get_var(name))
    }

    fn get_rng(&self) -> &RefCell<Pcg32> {
        self.base.get_rng()
    }
}

impl<C: ContextView> ContextView for PathEvalContext<'_, C> {}
