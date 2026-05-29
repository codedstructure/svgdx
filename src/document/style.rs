use crate::style::{Selectable, Stylable};
use crate::types::StyleMap;

use super::{EventKind, RawElement};

// TODO: need mut and non-mut versions of this
pub struct EventStyleWrapper<'a> {
    raw: &'a mut RawElement,
}

impl<'a> EventStyleWrapper<'a> {
    pub fn from_event(event: &'a mut EventKind) -> Option<Self> {
        match event {
            EventKind::Start(raw) | EventKind::Empty(raw) => Some(Self { raw }),
            _ => None,
        }
    }
}

impl Stylable for EventStyleWrapper<'_> {
    fn apply_styles(&mut self, styles: &StyleMap) {
        if styles.is_empty() {
            return;
        }
        for (key, value) in self.raw.get_attrs_mut().iter_mut() {
            if key == "style" {
                let existing = value.parse::<StyleMap>().unwrap_or_default();
                *value = existing.merge_styles(styles).to_string();
                return;
            }
        }

        self.raw
            .get_attrs_mut()
            .push(("style".to_string(), styles.to_string()));
    }
}

impl Selectable for EventStyleWrapper<'_> {
    fn name(&self) -> &str {
        self.raw.name()
    }

    fn get_classes(&self) -> Vec<String> {
        self.raw
            .get_attrs()
            .iter()
            .find(|(k, _)| k == "class")
            .map(|(_, v)| {
                v.split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
    }
}
