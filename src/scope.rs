use crate::elements::SvgElement;
use crate::types::{AttrMap, StyleMap, attr_split};

use std::collections::HashMap;

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
        if let Some(match_el) = &self.element
            && el.name() != *match_el
        {
            return false;
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

#[derive(Debug, Clone, Default)]
struct Scope {
    vars: HashMap<String, String>,
    defaults: Vec<(ElementMatch, SvgElement)>,
    /// indicates whether updates from this scope affect siblings
    /// rather than descendants, e.g. `<var>` or `<defaults>`
    pseudo: bool,
    /// indicates whether this scope is a `<specs>` block
    is_specs: bool,
}

impl Scope {
    fn from_element(el: &SvgElement) -> Self {
        // TODO: Current behaviour causes variables set in <loop> elements
        // to leak beyond '</loop>', not sure if that is ideal...
        let pseudo = matches!(el.name(), "var" | "varDefault" | "defaults" | "loop");
        let is_specs = el.name() == "specs";
        let vars = el.get_attrs().into_iter().collect();
        Self {
            vars,
            defaults: Vec::new(),
            pseudo,
            is_specs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopeStack {
    global: Scope,
    stack: Vec<Scope>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            global: Scope::default(),
            stack: Vec::new(),
        }
    }

    pub fn in_specs(&self) -> bool {
        self.stack.iter().any(|s| s.is_specs)
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &Scope> + '_ {
        std::iter::once(&self.global).chain(self.stack.iter().filter(|s| !s.pseudo))
    }

    fn var_scopes(&self) -> impl Iterator<Item = &Scope> + '_ {
        // Note the element we're currently processing should not be on the stack
        // so we can access variables of the same name, e.g. `<g x="2"/><rect x="$x"/></g>`
        // requires that when evaluating `x="$x"` we don't look up `x` in the
        // `rect` element itself.
        self.stack
            .iter()
            .rev()
            .skip(1)
            .chain(std::iter::once(&self.global))
    }

    /// Lookup variable in either parent attribute values or global variables
    /// set using the `<var>` element.
    pub fn get_var(&self, name: &str) -> Option<String> {
        for var_scope in self.var_scopes().map(|s| &s.vars) {
            if let Some(value) = var_scope.get(name) {
                return Some(value.to_string());
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn target_scope(&mut self) -> &mut Scope {
        self.stack
            .iter_mut()
            .rfind(|s| !s.pseudo)
            .unwrap_or(&mut self.global)
    }

    pub fn set_element_default(&mut self, el: &SvgElement) {
        let scope = self.target_scope();
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
        'outer: for scope in self.iter() {
            for (default, default_el) in &scope.defaults {
                if default.matches(el) {
                    let mut default_el = default_el.clone();
                    for (a_name, a_list, _, f) in &mut *augment_types {
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
        for (a_name, a_list, sep, f) in augment_types {
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
        self.target_scope()
            .vars
            .insert(name.to_string(), value.to_string());
    }

    pub fn update_current_scope(&mut self, el: &SvgElement) {
        if let Some(scope) = self.stack.last_mut() {
            *scope = Scope::from_element(el);
        }
    }

    pub fn push_element(&mut self, el: &SvgElement) {
        let scope = Scope::from_element(el);
        self.stack.push(scope);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }
}
