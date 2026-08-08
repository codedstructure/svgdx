use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::document::InputList;

/// Library of reuseable definitions, loaded from the `<defs>` of a source file.
// TODO: Arc because this (currently) needs to be Clone; should avoid
// TransformConfig being Clone and just clone a subset where required.
// TODO: mark used defs as used for later injection into output
#[derive(Clone)]
pub struct Library {
    pub name: String,
    pub defs: Arc<Vec<InputList>>,
    pub id_map: Arc<OnceLock<HashMap<String, SvgElement>>>,
}

impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library")
            .field("name", &self.name)
            .field("defs", &format!("{} entries", self.defs.len()))
            .finish()
    }
}

impl Library {
    pub fn lookup(&self, id: &str) -> Option<&SvgElement> {
        self.id_map.get_or_init(|| self.build_id_map()).get(id)
    }

    fn build_id_map(&self) -> HashMap<String, SvgElement> {
        let mut id_map = HashMap::new();

        for defs in self.defs.iter() {
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
                    id_map.insert(id.clone(), el);
                }
            }
        }

        id_map
    }
}

pub fn load_library(path: impl AsRef<Path>) -> Result<Library> {
    let path = path.as_ref();
    let content =
        fs::read_to_string(path).map_err(|e| Error::Document(format!("read {path:?}: {e}")))?;
    parse_library(content).map_err(|e| Error::Document(format!("parse {path:?}: {e}")))
}

fn parse_library(content: String) -> Result<Library> {
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
        defs: Arc::new(defs),
        id_map: Arc::new(OnceLock::new()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
