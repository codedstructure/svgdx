use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::document::InputList;

struct LibraryCache {
    /// element id -> SvgElement mapping for fragments in this library
    id_map: HashMap<String, SvgElement>,
    /// element id -> fragment index
    fragment_map: HashMap<String, usize>,
}

pub enum Fragment {
    Defs(InputList),
    Specs(InputList),
}

impl Fragment {
    fn events(&self) -> &InputList {
        match self {
            Self::Defs(events) | Self::Specs(events) => events,
        }
    }

    // specs fragments are never included in the output.
    fn is_output_fragment(&self) -> bool {
        matches!(self, Self::Defs(..))
    }
}

/// Library of reusable fragments, loaded from top-level `<defs>` / `<specs>` blocks.
pub struct Library {
    /// library name, from root `<svg name="...">` attribute
    pub name: String,
    /// entire event stream of library
    pub events: InputList,
    /// defs/specs fragments, in library document order
    pub fragments: Vec<Fragment>,
    /// cache of id -> SvgElement/fragment index, built on first lookup
    cache: OnceLock<LibraryCache>,
    // bitmap of which fragments have been used by `<use>` elements
    used_fragments: RefCell<Vec<bool>>,
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
        let Some(fragment_idx) = self
            .cache
            .get_or_init(|| self.build_cache())
            .fragment_map
            .get(id)
        else {
            return false;
        };
        if !self.fragments[*fragment_idx].is_output_fragment() {
            return false;
        }
        if let Some(used) = self.used_fragments.borrow_mut().get_mut(*fragment_idx) {
            *used = true;
        }
        true
    }

    /// list of fragments to inject in output document:
    ///  - only `<defs>` fragments are included, not `<specs>`
    ///  - only fragments marked as used by `<use>` elements are included
    pub fn output_fragments(&self) -> Vec<InputList> {
        self.used_fragments
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, used)| **used)
            .filter_map(|(idx, _)| match &self.fragments[idx] {
                Fragment::Defs(events) => Some(events.clone()),
                Fragment::Specs(..) => None,
            })
            .collect()
    }

    fn build_cache(&self) -> LibraryCache {
        let mut id_map = HashMap::new();
        let mut fragment_map = HashMap::new();

        for (fragment_idx, fragment) in self.fragments.iter().enumerate() {
            for event in fragment.events().iter() {
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
                    fragment_map.insert(id, fragment_idx);
                }
            }
        }

        LibraryCache {
            id_map,
            fragment_map,
        }
    }
}

pub fn parse_library(content: String) -> Result<Library> {
    let mut fragments = Vec::new();
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

    // top-level (under <svg> root) <defs> and <specs> blocks are considered library fragments.
    for fragment in events.iter().filter(|event| event.meta.depth == 1) {
        let Some(element) = fragment.element() else {
            continue;
        };

        let fragment_events = InputList::from(&events[fragment.event_range()]);
        match element.name() {
            "defs" => fragments.push(Fragment::Defs(fragment_events)),
            "specs" => {
                // skip custom-element specs blocks - those with 'element' attribute.
                if element.get_attrs().iter().any(|(key, _)| key == "element") {
                    continue;
                }
                fragments.push(Fragment::Specs(fragment_events));
            }
            _ => {}
        }
    }

    Ok(Library {
        name,
        used_fragments: RefCell::new(vec![false; fragments.len()]),
        events,
        fragments,
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
        assert_eq!(library.fragments.len(), 2);
        assert!(library.fragments[0].events().find("circle", None).is_some());
        assert!(library.fragments[1].events().find("rect", None).is_some());
    }

    #[test]
    fn test_parse_library_includes_skip_custom_element_specs() {
        let content = r#"
            <svg name="test">
                <specs>
                    <g id="s1" />
                </specs>
                <specs element="widget">
                    <g id="skip" />
                </specs>
                <defs>
                    <rect id="r1" />
                </defs>
            </svg>
        "#;
        let library = parse_library(content.to_string()).unwrap();

        assert_eq!(library.fragments.len(), 2);
        assert!(library.lookup("s1").is_some());
        assert!(library.lookup("r1").is_some());
        assert!(library.lookup("skip").is_none());
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
                    <specs>
                        <g id="tc">
                            <rect wh="10"/>
                            <circle cxy="5" r="5"/>
                        </g>
                    </specs>
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

    #[test]
    fn test_transform_reuse_omits_specs() {
        let config = config_with_library(
            r#"
                <svg name="lib">
                    <specs>
                        <g id="r1"><rect wh="10"/></g>
                    </specs>
                </svg>
            "#,
        );

        let output = transform_str(r##"<svg><reuse href="#lib:r1"/></svg>"##, &config).unwrap();

        assert!(output.contains(r#"<g class="r1"><rect width="10" height="10"/></g>"#));
        assert_eq!(output.matches("<specs").count(), 0);
        assert_eq!(output.matches("<defs").count(), 0);
    }
}
