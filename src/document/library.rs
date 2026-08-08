use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::document::InputList;

#[derive(Debug)]
struct LibraryCache {
    id_map: HashMap<String, SvgElement>,
    defs_map: HashMap<String, usize>,
}

/// Library of reuseable definitions, loaded from the `<defs>` of a source file.
pub struct Library {
    pub name: String,
    pub events: InputList,
    pub defs: Vec<InputList>,
    cache: OnceLock<LibraryCache>,
    used_defs: RefCell<Vec<bool>>,
}

impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library").field("name", &self.name).finish()
    }
}

impl Library {
    pub fn lookup(&self, id: &str) -> Option<&SvgElement> {
        self.cache.get_or_init(|| self.build_cache()).id_map.get(id)
    }

    pub fn mark_used(&self, id: &str) -> bool {
        let Some(defs_idx) = self
            .cache
            .get_or_init(|| self.build_cache())
            .defs_map
            .get(id)
        else {
            return false;
        };
        if let Some(used) = self.used_defs.borrow_mut().get_mut(*defs_idx) {
            *used = true;
        }
        true
    }

    pub fn used_defs(&self) -> Vec<InputList> {
        self.used_defs
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, used)| **used)
            .filter_map(|(idx, _)| self.defs.get(idx).cloned())
            .collect()
    }

    fn build_cache(&self) -> LibraryCache {
        let mut id_map = HashMap::new();
        let mut defs_map = HashMap::new();

        for (defs_idx, defs) in self.defs.iter().enumerate() {
            for event in defs.iter() {
                let Some(id) = event.element().and_then(|el| {
                    el.get_attrs()
                        .iter()
                        .find(|(key, _)| key == "id")
                        .map(|(_, id)| id.clone())
                }) else {
                    continue;
                };

                if let Ok(mut el) = SvgElement::try_from(event.clone()) {
                    el.library = Some(self.name.clone());
                    el.set_event_range((
                        event.meta.index,
                        event.meta.alt_idx.unwrap_or(event.meta.index),
                    ));
                    id_map.insert(id.clone(), el);
                    defs_map.insert(id, defs_idx);
                }
            }
        }

        LibraryCache { id_map, defs_map }
    }
}

pub fn parse_library(content: String) -> Result<Library> {
    let mut defs = Vec::new();
    let events: InputList = content.parse()?;

    let name = if let Some(root_svg) = events.find("svg", Some(0))
        && let Some(element) = root_svg.element()
        && let Some((_, name)) = element.get_attrs().iter().find(|(k, _)| k == "name")
    {
        name.clone()
    } else {
        return Err(Error::Document(
            "missing 'name' attribute on root <svg>".to_string(),
        ));
    };

    // top-level (under <svg> root) <defs> elements are considered part of the library
    for defs_instance in events.find_all("defs", Some(1)) {
        let inner_events = InputList::from(&events[defs_instance.event_range()]);
        defs.push(inner_events);
    }

    Ok(Library {
        name,
        used_defs: RefCell::new(vec![false; defs.len()]),
        events,
        defs,
        cache: OnceLock::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TransformConfig, transform_str};

    fn config_with_library(content: &str) -> TransformConfig {
        let mut config = TransformConfig::default();
        config.load_library(content).unwrap();
        config
    }

    #[test]
    fn test_parse_library() {
        let content = r#"
            <svg name="test">
                <defs>
                    <circle id="c1" />
                </defs>
                <defs>
                    <rect id="r1" />
                </defs>
            </svg>
        "#;
        let library = parse_library(content.to_string()).unwrap();
        assert_eq!(library.name, "test");
        assert_eq!(library.defs.len(), 2);
        assert!(library.defs[0].find("circle", None).is_some());
        assert!(library.defs[1].find("rect", None).is_some());
    }

    #[test]
    fn test_lookup_builds_cache_and_returns_stable_refs() {
        let content = r#"
            <svg name="test">
                <defs>
                    <circle id="c1" />
                    <rect id="r1" />
                </defs>
            </svg>
        "#;
        let library = parse_library(content.to_string()).unwrap();

        let first = library.lookup("c1").unwrap();
        let second = library.lookup("c1").unwrap();
        let rect = library.lookup("r1").unwrap();

        assert!(std::ptr::eq(first, second));
        assert_eq!(first.name(), "circle");
        assert_eq!(rect.name(), "rect");
        assert!(library.lookup("missing").is_none());
    }

    #[test]
    fn test_transform_reuse_from_included_library() {
        let config = config_with_library(
            r#"
                <svg name="lib">
                    <defs>
                        <g id="tc">
                            <rect wh="10"/>
                            <circle cxy="5" r="5"/>
                        </g>
                    </defs>
                </svg>
            "#,
        );

        let input = r##"<svg><reuse href="#lib:tc"/></svg>"##;
        let output = transform_str(input, &config).unwrap();

        assert!(output.contains(r#"<g class="tc">"#));
        assert!(output.contains(r#"<rect width="10" height="10"/>"#));
        assert!(output.contains(r#"<circle cx="5" cy="5" r="5"/>"#));
    }

    #[test]
    fn test_transform_library_use_rewrites_href_and_injects_defs() {
        let config = config_with_library(
            r#"
                <svg name="lib">
                    <defs><g id="tc"><rect wh="10"/><circle cxy="5" r="5"/></g></defs>
                </svg>
            "#,
        );

        let output = transform_str(r##"<svg><use href="#lib:tc"/></svg>"##, &config).unwrap();

        assert!(output.contains(r##"<use href="#tc"/>"##));
        assert!(
            output
                .contains(r#"<defs><g id="tc"><rect wh="10"/><circle cxy="5" r="5"/></g></defs>"#)
        );
        assert_eq!(output.matches("<defs").count(), 1);
    }

    #[test]
    fn test_transform_use_injects_used_defs_in_order() {
        let config = config_with_library(
            r#"
                <svg name="lib">
                    <defs><g id="first"><rect wh="10"/></g></defs>
                    <defs><g id="second"><circle cxy="5" r="5"/></g></defs>
                </svg>
            "#,
        );

        let single_output =
            transform_str(r##"<svg><use href="#lib:second"/></svg>"##, &config).unwrap();
        assert!(single_output.contains(r##"<use href="#second"/>"##));
        assert!(
            single_output.contains(r#"<defs><g id="second"><circle cxy="5" r="5"/></g></defs>"#)
        );
        assert!(!single_output.contains(r#"id="first""#));
        assert_eq!(single_output.matches("<defs").count(), 1);

        let both_output = transform_str(
            r##"<svg><use href="#lib:second"/><use href="#lib:first"/></svg>"##,
            &config,
        )
        .unwrap();
        let first_pos = both_output.find(r#"id="first""#).unwrap();
        let second_pos = both_output.find(r#"id="second""#).unwrap();
        assert!(first_pos < second_pos);
        assert_eq!(both_output.matches("<defs").count(), 2);
    }
}
