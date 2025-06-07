use crate::elements::{Element, ElementTransform, Layout, SvgElement};
use crate::errors::{Result, SvgdxError};
use crate::events::InputEvent;
use crate::expression::eval_attr;
use crate::geometry::BoundingBox;
use crate::types::{attr_split, extract_urlref, strp, AttrMap, ClassList, ElRef};
use crate::TransformConfig;

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::prelude::*;
use rand_pcg::Pcg32;

#[derive(Debug, Clone)]
struct ElementMatch {
    element: Option<String>,
    matches: Vec<String>,
    is_init: bool,
    is_final: bool,
}

impl ElementMatch {
    fn is_init(&self) -> bool {
        self.is_init
    }
    fn is_final(&self) -> bool {
        self.is_final
    }
    fn matches(&self, el: &SvgElement) -> bool {
        // early reject if element name doesn't match
        if let Some(match_el) = &self.element {
            if el.name != *match_el {
                return false;
            }
        }
        // early accept if there are no matches
        if self.matches.is_empty() {
            return true;
        }
        // otherwise iterate through matches
        for m in self.matches.iter() {
            if let Some((elem, class)) = m.split_once('.') {
                if (elem.is_empty() || elem == el.name) && el.has_class(class) {
                    return true;
                }
            } else if *m == el.name {
                return true;
            }
        }
        false
    }
}

impl From<&SvgElement> for ElementMatch {
    fn from(el: &SvgElement) -> Self {
        let element = if el.name == "_" {
            None
        } else {
            Some(el.name.clone())
        };
        let mut matches = Vec::new();
        let mut is_final = false;
        let mut is_init = false;
        if let Some(m) = el.get_attr("match") {
            for m in attr_split(m) {
                match m.as_str() {
                    "final" => is_final = true,
                    "init" => is_init = true,
                    _ => matches.push(m.to_string()),
                }
            }
        }
        Self {
            element,
            matches,
            is_init,
            is_final,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Scope {
    vars: HashMap<String, String>,
    defaults: Vec<(ElementMatch, SvgElement)>,
}

impl Scope {
    fn with_vars(vars: HashMap<String, String>) -> Self {
        Self {
            vars,
            ..Default::default()
        }
    }
}

pub struct TransformerContext {
    /// Current state of given element; may be updated as processing continues
    elem_map: HashMap<String, SvgElement>,
    /// Original state of given element; used for `reuse` elements
    original_map: HashMap<String, SvgElement>,
    /// Stack of elements which have been started but not yet ended
    ///
    /// Note empty elements are normally not pushed onto the stack,
    /// but `<reuse>` elements are an exception during processing of
    /// the referenced element.
    element_stack: Vec<SvgElement>,
    /// The element which `^` refers to; some elements are ignored as 'previous'
    prev_element: Option<SvgElement>,
    /// Stack of scoped variables etc
    scope_stack: Vec<Scope>,
    /// Pcg32 is used as it is both seedable and portable.
    rng: RefCell<Pcg32>,
    /// Current recursion depth
    current_depth: u32,
    /// Is this a 'real' SVG doc, or just a fragment?
    pub real_svg: bool,
    /// Are we in a <specs> block?
    pub in_specs: bool,
    /// The event-representation of the entire input SVG
    pub events: Vec<InputEvent>,
    /// id used by top-level SVG element if local_styles is true
    pub local_style_id: Option<String>,
    /// Config of transformer processing; updated by <config> elements
    pub config: TransformConfig,
}

impl Default for TransformerContext {
    fn default() -> Self {
        Self {
            elem_map: HashMap::new(),
            original_map: HashMap::new(),
            element_stack: Vec::new(),
            prev_element: None,
            scope_stack: Vec::new(),
            rng: RefCell::new(Pcg32::seed_from_u64(0)),
            local_style_id: None,
            current_depth: 0,
            real_svg: false,
            in_specs: false,
            events: Vec::new(),
            config: TransformConfig::default(),
        }
    }
}

pub trait ElementMap {
    fn get_element(&self, elref: &ElRef) -> Option<&SvgElement>;
    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>>;
}

pub trait VariableMap {
    fn get_var(&self, name: &str) -> Option<String>;
    fn get_rng(&self) -> &RefCell<Pcg32>;
}

pub trait ContextView: ElementMap + VariableMap {}

impl ElementMap for TransformerContext {
    fn get_element(&self, elref: &ElRef) -> Option<&SvgElement> {
        match elref {
            ElRef::Id(id) => self.elem_map.get(id),
            ElRef::Prev => self.prev_element.as_ref(),
        }
    }

    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>> {
        let target_el = el.get_target_element(self)?;
        let mut el_bbox = target_el.bbox()?;

        // TODO: move following to element::bbox() ?
        if el.name() == "use" || el.name() == "reuse" {
            // assumes el has already had position & attributes resolved
            let translate_x = el.get_attr("x");
            let translate_y = el.get_attr("y");
            if translate_x.is_some() || translate_y.is_some() {
                if let Some(ref mut bbox) = &mut el_bbox {
                    el_bbox = Some(bbox.translated(
                        translate_x.map(strp).unwrap_or(Ok(0.))?,
                        translate_y.map(strp).unwrap_or(Ok(0.))?,
                    ));
                }
            }
        }

        // TODO: this logic is duplicated in `impl EventGen for SvgElement` so
        // it works in both '^' contexts and root SVG bbox generation context.
        // Can't just move this to SvgElement::bbox() as it needs ElementMap.
        if let (Some(clip_path), Some(ref mut bbox)) = (el.get_attr("clip-path"), &mut el_bbox) {
            let clip_id = extract_urlref(clip_path).ok_or(SvgdxError::InvalidData(format!(
                "Invalid clip-path attribute: {clip_path}"
            )))?;
            let clip_el = self
                .get_element(&clip_id)
                .ok_or(SvgdxError::ReferenceError(clip_id))?;
            if let ("clipPath", Some(clip_bbox)) =
                (clip_el.name.as_str(), self.get_element_bbox(clip_el)?)
            {
                el_bbox = bbox.intersect(&clip_bbox);
            }
        }

        Ok(el_bbox)
    }
}

impl VariableMap for TransformerContext {
    /// Lookup variable in either parent attribute values or global variables
    /// set using the `<var>` element.
    fn get_var(&self, name: &str) -> Option<String> {
        // Note the element we're currently processing should not be on the stack
        // so we can access variables of the same name, e.g. `<g x="2"/><rect x="$x"/></g>`
        // requires that when evaluating `x="$x"` we don't look up `x` in the
        // `rect` element itself.
        for var_scope in self.scope_stack.iter().rev().map(|s| &s.vars) {
            if let Some(value) = var_scope.get(name) {
                return Some(value.to_string());
            }
        }
        None
    }

    fn get_rng(&self) -> &RefCell<Pcg32> {
        &self.rng
    }
}

impl ContextView for TransformerContext {}

impl TransformerContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new `TransformerContext` from a given config object.
    ///
    /// Note the config object is cloned and stored in the context.
    pub fn from_config(config: &TransformConfig) -> Self {
        let mut ctx = Self::default();
        ctx.set_config(config.clone());
        ctx
    }

    pub fn set_config(&mut self, config: TransformConfig) {
        self.seed_rng(config.seed);
        if config.use_local_styles {
            // randomise the local id to avoid conflicts with other SVG
            // elements in the same (e.g. HTML) document.
            let now_seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64;
            let mut rng = Pcg32::seed_from_u64(now_seed);
            self.local_style_id = Some(format!("svgdx-{:08x}", rng.random::<u32>()))
        } else {
            self.local_style_id = None;
        }
        self.config = config;
    }

    pub fn set_events(&mut self, events: Vec<InputEvent>) {
        self.events = events;
    }

    pub fn get_original_element(&self, elref: &ElRef) -> Option<&SvgElement> {
        match elref {
            ElRef::Id(id) => self.original_map.get(id),
            ElRef::Prev => self.prev_element.as_ref(),
        }
    }

    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = RefCell::new(Pcg32::seed_from_u64(seed));
    }

    fn ensure_scope(&mut self) -> &mut Scope {
        if self.scope_stack.is_empty() {
            let scope = Scope::default();
            self.scope_stack.push(scope);
        }

        self.scope_stack
            .last_mut()
            .expect("Scope-stack should be non-empty")
    }

    pub fn set_element_default(&mut self, el: &SvgElement) {
        let scope = self.ensure_scope();
        let el_match = ElementMatch::from(el);
        let mut mod_el = el.clone();
        for unwanted in &["id", "match"] {
            mod_el.pop_attr(unwanted);
        }
        scope.defaults.push((el_match, mod_el));
    }

    pub fn apply_defaults(&mut self, el: &mut SvgElement) {
        // Build up the default element we're going to apply until
        // we hit a `final` match.
        // Later attribute values override earlier ones; classes
        // are appended to existing classes.
        let mut classes = ClassList::new();
        let mut attrs = AttrMap::new();

        // For style, text-style and transform attributes we augment rather than
        // replace the existing value, similar to the behaviour of classes.
        let mut style_list = Vec::new();
        let mut text_style_list = Vec::new();
        let mut transform_list = Vec::new();
        let augment_types = &mut [
            // attribute name, value list, separator
            ("style", &mut style_list, "; "),
            ("text-style", &mut text_style_list, "; "),
            ("transform", &mut transform_list, " "),
        ];

        // Note we iterate through all scopes from outer inwards, updating
        // attributes as we go so the most local scope has highest priority.
        'outer: for scope in self.scope_stack.iter() {
            for (default, default_el) in &scope.defaults {
                if default.matches(el) {
                    let mut default_el = default_el.clone();
                    for (a_name, ref mut a_list, _) in &mut *augment_types {
                        if let Some(local) = default_el.pop_attr(a_name) {
                            a_list.push(local);
                        }
                    }
                    if default.is_init() {
                        classes = default_el.classes.clone();
                        attrs = default_el.attrs.clone();
                    } else {
                        classes.extend(&default_el.classes);
                        attrs.update(&default_el.attrs);
                    }
                    if default.is_final() {
                        break 'outer;
                    }
                }
            }
        }

        for (key, value) in &attrs {
            el.set_default_attr(key, value);
        }
        el.add_classes(&classes);

        // join style/transform attributes with the most local last
        for (a_name, ref mut a_list, sep) in augment_types {
            if !a_list.is_empty() {
                if let Some(local) = el.pop_attr(a_name) {
                    a_list.push(local);
                }
                let value = a_list.join(sep);
                // Note set_attr rather than set_default_attr as we replace
                // with newly constructed value
                el.set_attr(a_name, &value);
            }
        }
    }

    pub fn set_var(&mut self, name: &str, value: &str) {
        let scope = self.ensure_scope();
        scope.vars.insert(name.into(), value.into());
    }

    pub fn push_element(&mut self, el: &SvgElement) {
        let attrs = el.get_attrs();
        self.element_stack.push(el.clone());
        let scope = Scope::with_vars(attrs);
        self.scope_stack.push(scope);
    }

    pub fn pop_element(&mut self) -> Option<SvgElement> {
        self.scope_stack.pop();
        self.element_stack.pop()
    }

    pub fn inc_depth(&mut self) -> Result<()> {
        self.current_depth += 1;
        if self.current_depth > self.config.depth_limit {
            return Err(SvgdxError::DepthLimitExceeded(
                self.current_depth,
                self.config.depth_limit,
            ));
        }
        Ok(())
    }

    pub fn dec_depth(&mut self) -> Result<()> {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        } else {
            return Err(SvgdxError::from("Depth must be positive"));
        }
        Ok(())
    }

    pub fn get_top_element(&self) -> Option<SvgElement> {
        self.element_stack.last().cloned()
    }

    pub fn set_prev_element(&mut self, el: &SvgElement) {
        self.prev_element = Some(el.clone());
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            let id = eval_attr(id, self).unwrap_or_else(|_| id.to_string());
            if self.elem_map.insert(id.clone(), el.clone()).is_none() {
                self.original_map.insert(id, el.clone());
            }
        }
    }
}
