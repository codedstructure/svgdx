use crate::elements::{is_layout_element, SvgElement};
use crate::errors::{Error, Result};
use crate::events::InputEvent;
use crate::expr::eval_attr;
use crate::geometry::{BoundingBox, Size};
use crate::types::{attr_split, extract_urlref, strp, AttrMap, ElRef, OrderIndex, StyleMap};
use crate::TransformConfig;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
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
            if el.name() != *match_el {
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
                if (elem.is_empty() || elem == el.name()) && el.has_class(class) {
                    return true;
                }
            } else if *m == el.name() {
                return true;
            }
        }
        false
    }
}

impl From<&SvgElement> for ElementMatch {
    fn from(el: &SvgElement) -> Self {
        // attrs on the defaults element itself or on any
        // child element with name '_' will apply to all
        // element types, subject to `match`.
        let element = match el.name() {
            "_" | "defaults" => None,
            _ => Some(el.name().to_owned()),
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
    fn with_vars<K, V>(vars: &[(K, V)]) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let vars = vars
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();
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
    /// Tree of handled elements, used for previous element lookup
    index_map: BTreeMap<OrderIndex, SvgElement>,
    /// Current order index of the element being processed
    current_index: OrderIndex,
    /// Stack of scoped variables etc
    scope_stack: Vec<Scope>,
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
    /// id used by top-level SVG element if local_styles is true
    pub local_style_id: Option<String>,
    /// Config of transformer processing; updated by `<config>` elements
    pub config: TransformConfig,
}

impl Default for TransformerContext {
    fn default() -> Self {
        Self {
            elem_map: HashMap::new(),
            original_map: HashMap::new(),
            element_stack: Vec::new(),
            index_map: BTreeMap::new(),
            current_index: OrderIndex::new(0),
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
    #[allow(unused_variables)]
    fn set_current_element(&mut self, el: &SvgElement) {}
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
            ElRef::Id(id) => self.elem_map.get(id),
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
            ElRef::Prev(num) => self.get_element_offset(-(num.get() as isize)),
            ElRef::Next(num) => self.get_element_offset(num.get() as isize),
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
        // Later attribute values override earlier ones; classes and
        // style rules are appended to existing values.
        let mut classes = Vec::new();
        let mut attrs = AttrMap::new();
        let mut styles = StyleMap::new();

        type StoS = Box<dyn Fn(String) -> String>;
        fn rt_ts(ts: String) -> String {
            // Slight hack: round-trip text-style through parse/to_string
            // to de-duplicate any styles. This isn't ideal, but `text-style`
            // is just a normal attribute, unlike `style` which is special-cased
            // in `SvgElement`.
            ts.parse::<StyleMap>().map(|m| m.to_string()).unwrap_or(ts)
        }

        // For transform attributes we augment rather than replace,
        // similar to the behaviour of classes/styles.
        let mut transform_list = Vec::new();
        let mut text_style_list = Vec::new();
        let augment_types: &mut [(_, _, _, StoS)] = &mut [
            // attribute name, value list, separator, round-trip function
            ("text-style", &mut text_style_list, "; ", Box::new(rt_ts)),
            ("transform", &mut transform_list, " ", Box::new(|t| t)),
        ];

        // Note we iterate through all scopes from outer inwards, updating
        // attributes as we go so the most local scope has highest priority.
        'outer: for scope in self.scope_stack.iter() {
            for (default, default_el) in &scope.defaults {
                if default.matches(el) {
                    let mut default_el = default_el.clone();
                    for (a_name, ref mut a_list, _, f) in &mut *augment_types {
                        if let Some(local) = default_el.pop_attr(a_name) {
                            a_list.push(f(local));
                        }
                    }
                    if default.is_init() {
                        classes.clear();
                        attrs.clear();
                        styles.clear();
                    }
                    classes.extend(default_el.get_classes());
                    styles.extend(default_el.get_styles());
                    for (key, value) in default_el.get_attrs() {
                        attrs.insert(key, value);
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
        for c in classes.iter() {
            el.add_class(c);
        }

        let orig_styles = el.get_styles().clone();
        // tack original styles onto the end of the list to take priority
        for (s, v) in styles.iter().chain(orig_styles.iter()) {
            el.add_style(s, v);
        }

        // join style/transform attributes with the most local last
        for (a_name, ref mut a_list, sep, f) in augment_types {
            if !a_list.is_empty() {
                if let Some(local) = el.pop_attr(a_name) {
                    a_list.push(local);
                }
                let value = f(a_list.join(sep));
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
        let scope = Scope::with_vars(&attrs);
        self.scope_stack.push(scope);
    }

    pub fn pop_element(&mut self) -> Option<SvgElement> {
        self.scope_stack.pop();
        self.element_stack.pop()
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

    pub fn get_top_element(&self) -> Option<SvgElement> {
        self.element_stack.last().cloned()
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            let id = eval_attr(id, self).unwrap_or(id.to_string());
            if self.elem_map.insert(id.clone(), el.clone()).is_none() {
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
