use crate::TransformConfig;
use crate::document::InputEvent;
use crate::elements::{SvgElement, is_layout_element};
use crate::errors::{Error, Result};
use crate::expr::eval_attr;
use crate::geometry::{BoundingBox, Size, TransformAttr};
use crate::scope::ScopeStack;
use crate::types::{ElRef, OrderIndex, extract_urlref, strp};

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

use rand::prelude::*;
use rand_pcg::Pcg32;

pub struct TransformerContext {
    /// Map of element `id` to corresponding OrderIndex
    id_map: HashMap<String, OrderIndex>,
    /// Original state of given element; used for `reuse` elements
    original_map: HashMap<String, SvgElement>,
    /// Tree of handled elements by OrderIndex, may be updated during processing
    index_map: BTreeMap<OrderIndex, SvgElement>,
    /// Current order index of the element being processed
    current_index: OrderIndex,
    /// Stack of scopes (nested elements which have been started but not yet ended)
    ///
    /// Note empty elements are normally not pushed onto the stack, but `<reuse>`
    /// elements are an exception during processing of the referenced element.
    scope_stack: ScopeStack,
    /// Pcg32 is used as it is both seedable and portable.
    rng: RefCell<Pcg32>,
    /// Current recursion depth
    current_depth: u32,
    /// Is this a 'real' SVG doc, or just a fragment?
    pub real_svg: bool,
    /// Are we in a `<specs>` block?
    pub in_specs: bool,
    /// The event-representation of the entire input SVG
    pub events: Vec<InputEvent>,
    /// Config of transformer processing; updated by `<config>` elements
    pub config: TransformConfig,
    /// Set of custom element names registered via `<specs element="...">`
    named_spec_map: HashSet<String>,
}

impl Default for TransformerContext {
    fn default() -> Self {
        Self {
            original_map: HashMap::new(),
            index_map: BTreeMap::new(),
            id_map: HashMap::new(),
            current_index: OrderIndex::new(0),
            scope_stack: ScopeStack::new(),
            rng: RefCell::new(Pcg32::seed_from_u64(0)),
            current_depth: 0,
            real_svg: false,
            in_specs: false,
            events: Vec::new(),
            config: TransformConfig::default(),
            named_spec_map: HashSet::new(),
        }
    }
}

pub trait ElementMap {
    fn set_current_element(&mut self, _el: &SvgElement);
    fn get_element(&self, elref: &ElRef) -> Option<&SvgElement>;
    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>>;
    fn get_element_size(&self, el: &SvgElement) -> Result<Option<Size>>;
    fn get_target_element(&self, el: &SvgElement) -> Result<SvgElement> {
        Ok(el.clone())
    }
}

pub trait VariableMap {
    fn get_var(&self, name: &str) -> Option<String>;
    fn get_rng(&self) -> &RefCell<Pcg32>;
}

pub trait ConfigView {
    fn config(&self) -> &TransformConfig;
}

pub trait ContextView: ElementMap + VariableMap {}

impl ElementMap for TransformerContext {
    /// mark the current element as being processed.
    ///
    /// used when determining relative ElRef offsets.
    fn set_current_element(&mut self, el: &SvgElement) {
        self.current_index = el.order_index.clone();
        self.index_map.insert(el.order_index.clone(), el.clone());
    }

    fn get_element(&self, elref: &ElRef) -> Option<&SvgElement> {
        match elref {
            ElRef::Id(id) => self.id_map.get(id).and_then(|oi| self.index_map.get(oi)),
            ElRef::Prev(num) => self.get_element_offset(-(num.get() as isize)),
            ElRef::Next(num) => self.get_element_offset(num.get() as isize),
        }
    }

    fn get_element_size(&self, el: &SvgElement) -> Result<Option<Size>> {
        let target_el = self.get_target_element(el)?;
        let el_size = target_el.size(self)?;

        Ok(el_size)
    }

    fn get_target_element(&self, el: &SvgElement) -> Result<SvgElement> {
        use crate::types::OrderIndex; // used for circular reference detection

        // TODO: this uses OrderIndex to uniquely identify elements, but that's a bit
        // of a hack. In particular using `id` or `href` is insufficient, as doesn't
        // cope with '^' where the target might not even have an id. Would be better
        // to assign a dedicated internal ID to every element and use that.
        let mut seen: Vec<OrderIndex> = vec![];
        let mut element = el;

        while let "use" | "reuse" = element.name() {
            let href = element
                .get_attr("href")
                .ok_or_else(|| Error::MissingAttr("href".to_owned()))?;
            let elref = href.parse()?;
            if let Some(el) = self.get_element(&elref) {
                if seen.contains(&el.order_index) {
                    return Err(Error::CircularRef(format!("{elref} already seen")));
                }
                seen.push(el.order_index.clone());
                element = el;
            } else {
                return Err(Error::Reference(elref));
            }
        }
        Ok(element.clone())
    }

    fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>> {
        let target_el = self.get_target_element(el)?;
        let mut el_bbox = target_el.bbox()?;

        // TODO: move following to element::bbox() ?
        if let "use" | "reuse" = el.name() {
            // assumes el has already had position & attributes resolved
            let translate_x = el.get_attr("x");
            let translate_y = el.get_attr("y");
            if (translate_x.is_some() || translate_y.is_some())
                && let Some(bbox) = &mut el_bbox
            {
                el_bbox = Some(bbox.translated(
                    translate_x.map(strp).unwrap_or(Ok(0.))?,
                    translate_y.map(strp).unwrap_or(Ok(0.))?,
                ));
            }
        }

        // TODO: this logic is duplicated in `impl EventGen for SvgElement` so
        // it works in both '^' contexts and root SVG bbox generation context.
        // Can't just move this to SvgElement::bbox() as it needs ElementMap.
        if let (Some(clip_path), Some(bbox)) = (el.get_attr("clip-path"), &mut el_bbox) {
            let clip_id = extract_urlref(clip_path)
                .ok_or_else(|| Error::InvalidValue("clip-path".into(), clip_path.into()))?;
            let clip_el = self
                .get_element(&clip_id)
                .ok_or_else(|| Error::Reference(clip_id))?;
            if let ("clipPath", Some(clip_bbox)) = (clip_el.name(), self.get_element_bbox(clip_el)?)
            {
                el_bbox = bbox.intersect(&clip_bbox);
            }
        }

        // determine how many levels up we need to go to find common ancestor. OrderIndex is already
        // a path from root to element, so we can use that and see where they diverge.
        // TODO: this (probably) assumes that current is 'untransformed' relative to target;
        // may need to apply inverse transforms on current heading up to common ancestor, then
        // apply target transforms from there down to target?
        if let Some(bbox) = &mut el_bbox {
            let common_prefix = self.current_index.common_prefix(&target_el.order_index);
            // examine each element from common parent down to target
            // apply any transforms beyond common ancestor
            for oi in target_el
                .order_index
                .ancestors()
                .iter()
                .skip(common_prefix.depth())
            {
                if let Some(el) = self.index_map.get(oi) {
                    // any positional attrs on a group imply it hasn't yet been
                    // processed into a transform yet.
                    if el.pos_by_transform() && el.has_pos_attrs() {
                        // hasn't yet been expanded -> reference error.
                        return Err(Error::MissingBBox(format!(
                            "Element at {} has unexpanded xy attribute",
                            oi
                        )));
                    }
                    if let Some(xfrm_attr) = el.get_attr("transform") {
                        let xfrm: TransformAttr = xfrm_attr.parse().unwrap_or_default();
                        *bbox = xfrm.apply(bbox)?;
                    }
                }
            }
        }

        Ok(el_bbox)
    }
}

impl VariableMap for TransformerContext {
    /// Lookup variable in either parent attribute values or global variables
    /// set using the `<var>` element.
    fn get_var(&self, name: &str) -> Option<String> {
        self.scope_stack.get_var(name)
    }

    fn get_rng(&self) -> &RefCell<Pcg32> {
        &self.rng
    }
}

impl ConfigView for TransformerContext {
    fn config(&self) -> &TransformConfig {
        &self.config
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
        for (k, v) in &config.vars {
            self.set_var(k.as_str(), v.as_str());
        }
        self.config = config;
    }

    pub fn set_events(&mut self, events: Vec<InputEvent>) {
        self.events = events;
    }

    pub fn get_original_element(&self, elref: &ElRef) -> Option<&SvgElement> {
        match elref {
            ElRef::Id(id) => self.original_map.get(id),
            ElRef::Prev(num) => self.get_element_offset(-(num.get() as isize)),
            ElRef::Next(num) => self.get_element_offset(num.get() as isize),
        }
    }

    pub fn register_named_spec(&mut self, name: String, el: SvgElement) {
        self.original_map.entry(name.clone()).or_insert(el);
        self.named_spec_map.insert(name);
    }

    pub fn is_named_spec(&self, name: &str) -> bool {
        self.named_spec_map.contains(name)
    }

    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = RefCell::new(Pcg32::seed_from_u64(seed));
    }

    pub fn set_element_default(&mut self, el: &SvgElement) {
        self.scope_stack.set_element_default(el);
    }

    pub fn apply_defaults(&mut self, el: &mut SvgElement) {
        self.scope_stack.apply_defaults(el);
    }

    pub fn set_var(&mut self, name: &str, value: &str) {
        self.scope_stack.set_var(name, value);
    }

    pub fn set_var_default(&mut self, name: &str, value: &str) {
        // need a full 'get_var' search through scopes to check if already set.
        if self.get_var(name).is_none() {
            self.set_var(name, value);
        }
    }

    pub fn push_element(&mut self, el: &SvgElement) {
        self.scope_stack.push_element(el);
    }

    pub fn pop_element(&mut self) {
        self.scope_stack.pop();
    }

    pub fn is_top_level(&self) -> bool {
        self.scope_stack.is_empty()
    }

    pub fn inc_depth(&mut self) -> Result<()> {
        self.current_depth += 1;
        if self.current_depth > self.config.depth_limit {
            return Err(Error::DepthLimit(
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
            return Err(Error::InternalLogic("dec_depth underflow".into()));
        }
        Ok(())
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            let id = eval_attr(id, self).unwrap_or(id.to_string());
            if self
                .id_map
                .insert(id.clone(), el.order_index.clone())
                .is_none()
            {
                self.original_map.insert(id, el.clone());
            }
        }
        self.set_current_element(el);
    }

    fn get_element_offset(&self, offset: isize) -> Option<&SvgElement> {
        let current = &self.current_index;
        if offset == 0 {
            return self.index_map.get(current);
        }

        // first element in a container etc should be able to reference the
        // previous element which will be at a higher level (lower depth).  but
        // first element *after* a container should not be able to see something
        // *inside* that container.  Loops / if / etc shouldn't count as
        // descending...

        if offset > 0 {
            self.index_map
                .range(current..)
                .filter(|(oi, _)| oi.depth() <= current.depth())
                .filter(|(_, el)| is_layout_element(el))
                .nth(offset as usize)
                .map(|(_, value)| value)
        } else {
            self.index_map
                .range(..current)
                .rev()
                .filter(|(oi, _)| oi.depth() <= current.depth())
                .filter(|(_, el)| is_layout_element(el))
                // when scanning backwards, ignore any parent elements, e.g. a <g> we're inside of
                .filter(|(oi, _)| !current.has_prefix(oi))
                .nth((-offset - 1) as usize)
                .map(|(_, value)| value)
        }
    }
}
