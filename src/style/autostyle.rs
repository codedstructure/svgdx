use std::borrow::Cow;
use std::collections::HashMap;

use crate::elements::SvgElement;
use crate::types::StyleMap;

use super::rules;
use super::types::{Rule, Selectable, Selected, Stylable};
use super::{omap::InsertOrderMap, ContextTheme};

impl Selectable for SvgElement {
    fn name(&self) -> &str {
        self.name()
    }

    fn get_classes(&self) -> Vec<String> {
        self.get_classes()
    }
}

impl Stylable for SvgElement {
    fn apply_styles(&mut self, styles: &StyleMap) {
        self.apply_auto_styles(styles);
    }
}

pub(super) trait StyleProvider {
    fn new(theme: &ContextTheme) -> Self
    where
        Self: Sized;

    // Set of rules for this provider.
    fn get_rules(&self) -> Vec<Rule>;

    // If any matches are found, also include these styles.
    fn group_styles(&self) -> Vec<String> {
        Vec::new()
    }

    // Allows provider to update internal state and/or veto matches.
    fn on_match(&mut self, _rule: &Rule, _selected: &Selected) -> bool {
        true
    }

    fn group_defs(&self) -> Vec<String> {
        Vec::new()
    }

    fn eval_value<'a>(&self, value: &'a str, _s: &Selected) -> Cow<'a, str> {
        Cow::Borrowed(value)
    }
}

pub struct StyleRegistry {
    tb: ContextTheme,
    rules: Vec<Box<dyn StyleProvider>>,
}

impl Default for StyleRegistry {
    fn default() -> Self {
        let mut r = StyleRegistry {
            tb: ContextTheme::default(),
            rules: Vec::new(),
        };
        r.register_all();
        r
    }
}

impl StyleRegistry {
    pub fn new(tb: &ContextTheme) -> Self {
        StyleRegistry {
            tb: tb.clone(),
            rules: Vec::new(),
        }
    }

    pub fn register_all(&mut self) {
        self.register(rules::DefaultStyles::new(&self.tb));
        self.register(rules::ColourStyles::new(&self.tb));
        self.register(rules::StrokeWidthStyles::new(&self.tb));
        self.register(rules::TextStyles::new(&self.tb));
        self.register(rules::ArrowStyles::new(&self.tb));
        self.register(rules::DashStyles::new(&self.tb));
        self.register(rules::PatternStyles::new(&self.tb));
        self.register(rules::ShadowStyles::new(&self.tb));
    }

    fn register(&mut self, provider: impl StyleProvider + 'static) {
        self.rules
            .push(Box::new(provider) as Box<dyn StyleProvider>);
    }

    pub fn generate_css<E: Selectable>(&mut self, elements: &[&E]) -> (Vec<String>, Vec<String>) {
        let mut style = Vec::new();
        let mut defs = Vec::new();

        for provider in &mut self.rules {
            let mut selector_style_map = InsertOrderMap::new();
            let rules = provider.get_rules();
            for element in elements {
                for rule in &rules {
                    let mut stylemap = StyleMap::new();
                    if let Some(selected) = rule.selector.matches(*element) {
                        if !provider.on_match(rule, &selected) {
                            continue; // Skip if on_match vetoes the match
                        }
                        for style_item in &rule.styles {
                            let value = provider.eval_value(&style_item.value, &selected);
                            stylemap.insert(style_item.key.clone(), value.to_string());
                        }
                        if let Some(mr) = selected.match_result() {
                            selector_style_map
                                .get_or_insert_with_mut(
                                    rule.selector.to_css_selector(mr),
                                    StyleMap::new,
                                )
                                .extend(&stylemap);
                        }
                    }
                }
            }

            defs.extend(provider.group_defs());
            style.extend(provider.group_styles());
            for (selector, styles) in selector_style_map {
                style.push(format!("{selector} {{ {styles} }}"));
            }
        }

        (style, defs)
    }

    pub fn update_elements<E: Selectable + Stylable>(
        &mut self,
        elements: &mut [&mut E],
    ) -> (Vec<String>, Vec<String>) {
        let mut defs = Vec::new();
        let mut style = Vec::new();
        // NOTE: want provider on the outside to deal with group_styles/group_defs,
        // but also ideally have element on the outside to deal with having per-element
        // accumulated styles. Resolve by having enumerating the elements list and
        // using index into a mapping. Probably scope for tidying this up.
        let mut el_style_mapping = HashMap::with_capacity(elements.len());
        for provider in &mut self.rules {
            let rules = provider.get_rules();
            for (el_idx, element) in elements.iter_mut().enumerate() {
                // get el_style_mapping entry for this element
                let el_styles = el_style_mapping.entry(el_idx).or_insert_with(StyleMap::new);
                for rule in &rules {
                    if let Some(selected) = rule.selector.matches(*element) {
                        if !provider.on_match(rule, &selected) {
                            continue; // Skip if on_match vetoes the match
                        }
                        for style_item in &rule.styles {
                            let value = provider.eval_value(&style_item.value, &selected);
                            // element.add_style(&style_item.key, &value);
                            el_styles.insert(style_item.key.clone(), value.to_string());
                        }
                    }
                }
            }
            defs.extend(provider.group_defs());
            style.extend(provider.group_styles());
        }
        for (el_idx, element) in elements.iter_mut().enumerate() {
            if let Some(el_styles) = el_style_mapping.get(&el_idx) {
                element.apply_styles(el_styles);
            }
        }
        (style, defs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)] //, PartialEq, Eq)]
    struct MockElement {
        name: String,
        classes: Vec<String>,
        style: StyleMap,
    }

    impl MockElement {
        fn new(name: &str, classes: &[&str]) -> Self {
            MockElement {
                name: name.to_string(),
                classes: classes.iter().map(|c| c.to_string()).collect(),
                style: StyleMap::new(),
            }
        }
    }

    impl Selectable for MockElement {
        fn name(&self) -> &str {
            &self.name
        }

        fn get_classes(&self) -> Vec<String> {
            self.classes.clone()
        }
    }

    impl Stylable for MockElement {
        fn apply_styles(&mut self, styles: &StyleMap) {
            for (key, value) in styles {
                self.style.insert(key.to_string(), value.to_string());
            }
        }
    }

    #[test]
    fn test_registry() {
        let mut registry = StyleRegistry::default();
        registry.register(rules::TextStyles::new(&ContextTheme::default()));
        registry.register(rules::StrokeWidthStyles::new(&ContextTheme::default()));
        registry.register(rules::ArrowStyles::new(&ContextTheme::default()));

        let a = MockElement::new("text", &["d-text", "d-text-bold"]);
        let b = MockElement::new("text", &["d-text", "d-text-italic"]);
        let c = MockElement::new("text", &["d-text", "d-text-bold"]);
        let (styles, _defs) = registry.generate_css(&[&a, &b, &c]);
        let styles = styles.join("\n");
        assert!(styles.contains(&"font-weight: bold".to_string()));
    }

    #[test]
    fn test_stylable() {
        let mut element = MockElement::new("rect", &["d-fill-red"]);

        let mut registry = StyleRegistry::default();
        registry.register(rules::ColourStyles::new(&ContextTheme::default()));
        registry.update_elements(&mut [&mut element]);

        assert_eq!(element.style.get("fill"), Some("red"));
    }

    #[test]
    fn test_shadow() {
        let element = MockElement::new("rect", &["d-hardshadow"]);

        let mut registry = StyleRegistry::default();
        let (css, defs) = registry.generate_css(&[&element]);

        println!("CSS: {}", css.join("\n"));
        println!("Defs: {}", defs.join("\n"));
    }
}
