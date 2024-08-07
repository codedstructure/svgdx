use crate::element::SvgElement;
use crate::events::InputEvent;
use crate::expression::eval_attr;
use crate::transform::ElementLike;
use crate::TransformConfig;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rand::prelude::*;

pub struct TransformerContext {
    // Current state of given element; may be updated as processing continues
    elem_map: HashMap<String, SvgElement>,
    // Original state of given element; used for `reuse` elements
    original_map: HashMap<String, SvgElement>,
    // Stack of elements which have been started but not yet ended
    // Note empty elements are normally not pushed onto the stack,
    // but `<reuse>` elements are an exception during processing of
    // the referenced element.
    element_stack: Vec<Rc<RefCell<dyn ElementLike>>>,
    // The element which `^` refers to; some elements are ignored as 'previous'
    prev_element: Option<SvgElement>,
    // Current variable values
    pub variables: HashMap<String, String>,
    // SmallRng is used as it is seedable.
    rng: RefCell<SmallRng>,
    // Is this a 'real' SVG doc, or just a fragment?
    pub real_svg: bool,
    // Are we in a <specs> block?
    pub in_specs: bool,
    // How many <loop> elements deep are we?
    pub loop_depth: usize,
    // The event-representation of the entire input SVG
    pub events: Vec<InputEvent>,
    // Config of transformer processing; updated by <config> elements
    pub config: TransformConfig,
}

impl Default for TransformerContext {
    fn default() -> Self {
        Self {
            elem_map: HashMap::new(),
            original_map: HashMap::new(),
            element_stack: Vec::new(),
            prev_element: None,
            variables: HashMap::new(),
            rng: RefCell::new(SmallRng::seed_from_u64(0)),
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
}

pub trait VariableMap {
    fn get_var(&self, name: &str) -> Option<String>;
    fn get_rng(&self) -> &RefCell<SmallRng>;
}

pub trait ContextView: ElementMap + VariableMap {}

impl ElementMap for TransformerContext {
    fn get_element(&self, id: &str) -> Option<&SvgElement> {
        self.elem_map.get(id)
    }

    fn get_prev_element(&self) -> Option<&SvgElement> {
        self.prev_element.as_ref()
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
        for element_scope in self.element_stack.iter().rev() {
            if let Some(Some(value)) = element_scope
                .borrow()
                .get_element()
                .map(|el| el.get_attr(name))
            {
                return Some(value.to_string());
            }
        }
        return self.variables.get(name).cloned();
    }

    fn get_rng(&self) -> &RefCell<SmallRng> {
        &self.rng
    }
}

impl ContextView for TransformerContext {}

impl TransformerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_events(&mut self, events: Vec<InputEvent>) {
        self.events = events;
    }

    pub fn get_original_element(&self, id: &str) -> Option<&SvgElement> {
        self.original_map.get(id)
    }

    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = RefCell::new(SmallRng::seed_from_u64(seed));
    }

    #[cfg(test)]
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.insert(name.into(), value.into());
    }

    pub fn push_element(&mut self, ell: Rc<RefCell<dyn ElementLike>>) {
        self.element_stack.push(ell);
    }

    pub fn pop_element(&mut self) -> Option<Rc<RefCell<dyn ElementLike>>> {
        self.element_stack.pop()
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
