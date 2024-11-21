use crate::element::SvgElement;
use crate::events::InputEvent;
use crate::expression::eval_attr;
use crate::position::BoundingBox;
use crate::transform::ElementLike;
use crate::transform_attr::TransformAttr;
use crate::types::strp;
use crate::TransformConfig;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::prelude::*;
use rand_pcg::Pcg32;

use anyhow::{bail, Context, Result};

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
    element_stack: Vec<Rc<RefCell<dyn ElementLike>>>,
    /// The element which `^` refers to; some elements are ignored as 'previous'
    prev_element: Option<SvgElement>,
    /// Stack of scoped variable mappings
    var_stack: Vec<HashMap<String, String>>,
    /// Pcg32 is used as it is both seedable and portable.
    rng: RefCell<Pcg32>,
    /// Is this a 'real' SVG doc, or just a fragment?
    pub real_svg: bool,
    /// Are we in a <specs> block?
    pub in_specs: bool,
    /// How many <loop> elements deep are we?
    pub loop_depth: usize,
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
            var_stack: Vec::new(),
            rng: RefCell::new(Pcg32::seed_from_u64(0)),
            local_style_id: None,
            real_svg: false,
            in_specs: false,
            loop_depth: 0,
            events: Vec::new(),
            config: TransformConfig::default(),
        }
    }
}

pub trait ElementMap {
    fn get_element(&self, id: &str) -> Option<&SvgElement>;
    fn get_prev_element(&self) -> Option<&SvgElement>;
    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>>;
}

pub trait VariableMap {
    fn get_var(&self, name: &str) -> Option<String>;
    fn get_rng(&self) -> &RefCell<Pcg32>;
}

pub trait ContextView: ElementMap + VariableMap {}

impl ElementMap for TransformerContext {
    fn get_element(&self, id: &str) -> Option<&SvgElement> {
        self.elem_map.get(id)
    }

    fn get_prev_element(&self) -> Option<&SvgElement> {
        self.prev_element.as_ref()
    }

    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>> {
        // This is recursive for use/reuse elements. We use an inner function and a vec of hrefs
        // to detect circular references.
        fn inner(
            el: &SvgElement,
            ctx: &TransformerContext,
            already: &mut Vec<String>,
        ) -> Result<Option<BoundingBox>> {
            let mut el_bbox = if el.name == "use" || el.name == "reuse" {
                // use and reuse elements reference another element - get the bbox of the target
                // (which could be another (re)use element)
                let href = el
                    .get_attr("href")
                    .and_then(|href| href.strip_prefix('#').map(|s| s.to_string()))
                    .context("Could not determine href for element")?;

                if already.contains(&href) {
                    bail!("Circular reference: {}", href);
                }
                already.push(href.clone());

                let target_el = ctx
                    .get_element(&href)
                    .context("Could not find element with id")?;
                // recurse to get bbox of the target
                inner(target_el, ctx, already)?
            } else {
                el.bbox()?
            };
            if el.name == "use" || el.name == "reuse" {
                let translate_x = el.get_attr("x").map(|x| eval_attr(&x, ctx));
                let translate_y = el.get_attr("y").map(|y| eval_attr(&y, ctx));
                if translate_x.is_some() || translate_y.is_some() {
                    if let Some(ref mut bbox) = &mut el_bbox {
                        el_bbox = Some(bbox.translated(
                            translate_x.map(|tx| strp(&tx)).unwrap_or(Ok(0.))?,
                            translate_y.map(|ty| strp(&ty)).unwrap_or(Ok(0.))?,
                        ));
                    }
                }
            }
            if let Some(transform) = el.get_attr("transform") {
                let transform: TransformAttr = transform.parse()?;
                if let Some(ref mut bbox) = &mut el_bbox {
                    el_bbox = Some(transform.apply(bbox));
                }
            }
            Ok(el_bbox)
        }
        let mut already_seen = Vec::new();
        inner(el, self, &mut already_seen)
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
        for var_scope in self.var_stack.iter().rev() {
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
            self.local_style_id = Some(format!("svgdx-{:08x}", rng.gen::<u32>()))
        } else {
            self.local_style_id = None;
        }
        self.config = config;
    }

    pub fn set_events(&mut self, events: Vec<InputEvent>) {
        self.events = events;
    }

    pub fn get_original_element(&self, id: &str) -> Option<&SvgElement> {
        self.original_map.get(id)
    }

    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = RefCell::new(Pcg32::seed_from_u64(seed));
    }

    pub fn set_var(&mut self, name: &str, value: &str) {
        if let Some(scope) = self.var_stack.last_mut() {
            // There's no scope yet; create one
            scope.insert(name.into(), value.into());
        } else {
            let mut scope = HashMap::new();
            scope.insert(name.into(), value.into());
            self.var_stack.push(scope);
        }
    }

    pub fn push_element(&mut self, ell: Rc<RefCell<dyn ElementLike>>) {
        let attrs = if let Some(element) = ell.borrow().get_element() {
            element.get_attrs()
        } else {
            HashMap::new()
        };
        self.element_stack.push(ell);
        self.var_stack.push(attrs);
    }

    pub fn pop_element(&mut self) -> Option<Rc<RefCell<dyn ElementLike>>> {
        self.var_stack.pop();
        self.element_stack.pop()
    }

    pub fn get_closure(&self) -> HashMap<String, String> {
        let mut closure = HashMap::new();
        for var_scope in &self.var_stack {
            for (k, v) in var_scope {
                closure.insert(k.clone(), v.clone());
            }
        }
        closure
    }

    pub fn set_closure(&mut self, c: HashMap<String, String>) {
        self.var_stack.push(c.clone());
    }

    pub fn pop_closure(&mut self) {
        self.var_stack.pop();
    }

    pub fn get_top_element(&self) -> Option<Rc<RefCell<dyn ElementLike>>> {
        self.element_stack.last().cloned()
    }

    pub fn set_prev_element(&mut self, el: SvgElement) {
        self.prev_element = Some(el);
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            let id = eval_attr(&id, self);
            if self.elem_map.insert(id.clone(), el.clone()).is_none() {
                self.original_map.insert(id, el.clone());
            }
        }
    }
}
