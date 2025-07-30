use super::colours::COLOUR_LIST;

pub trait Selectable {
    fn name(&self) -> &str;
    fn get_classes(&self) -> Vec<String>;
}

pub trait Stylable {
    fn add_style(&mut self, key: &str, value: &str);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    pub key: String,
    pub value: String,
}

impl Style {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Style {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    // Matches given element type
    Element(MatchType),
    // Matches text and tspan; used for text styling
    TextLike(MatchType),
    // Matches all elements with a given class
    Class(MatchType),
    // Matches lines, polylines, and paths
    LineLike(MatchType),
    // Matches basic shapes; used for stroke/fill styles
    BasicShape(MatchType),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selected {
    Element(MatchResult),
    TextLike(MatchResult),
    Class(MatchResult),
    LineLike(MatchResult),
    BasicShape(MatchResult),
}

impl Selected {
    pub fn as_class(&self) -> Option<String> {
        match self {
            Selected::Element(_) => None, // Element does not have a class
            Selected::TextLike(mr) => mr.as_class(),
            Selected::Class(mr) => mr.as_class(),
            Selected::LineLike(mr) => mr.as_class(),
            Selected::BasicShape(mr) => mr.as_class(),
        }
    }

    pub fn match_result(&self) -> Option<&MatchResult> {
        match self {
            Selected::Element(mr) => Some(mr),
            Selected::TextLike(mr) => Some(mr),
            Selected::Class(mr) => Some(mr),
            Selected::LineLike(mr) => Some(mr),
            Selected::BasicShape(mr) => Some(mr),
        }
    }
}

impl Selector {
    pub fn matches<E: Selectable>(&self, element: &E) -> Option<Selected> {
        let name = element.name();
        let classes = element.get_classes();
        match self {
            Selector::Element(mt) => match mt {
                MatchType::Any => Some(Selected::Element(MatchResult::Any)),
                MatchType::Element(el) if name == el => {
                    Some(Selected::Element(MatchResult::Element(name.to_string())))
                }
                _ => None,
            },
            Selector::TextLike(mt) => {
                if name == "text" || name == "tspan" {
                    if mt == &MatchType::Any {
                        return Some(Selected::TextLike(MatchResult::Any));
                    }
                    for class in classes {
                        if let Some(mr) = mt.matches_class(&class) {
                            return Some(Selected::TextLike(mr));
                        }
                    }
                }
                None
            }
            Selector::Class(mt) => {
                for class in classes {
                    if let Some(mr) = mt.matches_class(&class) {
                        return Some(Selected::Class(mr));
                    }
                }
                None
            }
            Selector::LineLike(mt) => {
                if name == "line" || name == "polyline" || name == "path" {
                    if mt == &MatchType::Any {
                        return Some(Selected::LineLike(MatchResult::Any));
                    }
                    for class in classes {
                        if let Some(mr) = mt.matches_class(&class) {
                            return Some(Selected::LineLike(mr));
                        }
                    }
                }
                None
            }
            Selector::BasicShape(mt) => {
                if name == "rect" || name == "circle" || name == "ellipse" || name == "polygon" {
                    if mt == &MatchType::Any {
                        return Some(Selected::BasicShape(MatchResult::Any));
                    }
                    for class in classes {
                        if let Some(mr) = mt.matches_class(&class) {
                            return Some(Selected::BasicShape(mr));
                        }
                    }
                }
                None
            }
        }
    }

    pub fn to_css_selector(&self, mr: &MatchResult) -> String {
        if let Some(class) = mr.as_class() {
            match self {
                Selector::Element(_) => unreachable!("Element::as_class() will return None"),
                Selector::TextLike(_) => format!("text.{class}, tspan.{class}, text.{class} *"),
                Selector::Class(_) => format!(".{class}"),
                Selector::LineLike(_) => format!("line.{class}, polyline.{class}, path.{class}"),
                Selector::BasicShape(_) => {
                    format!("rect.{class}, circle.{class}, ellipse.{class}, polygon.{class}")
                }
            }
        } else if let Some(el) = mr.as_element() {
            // The element name is the selector
            el.to_string()
        } else if mr == &MatchResult::Any {
            match self {
                Selector::Element(_) => unreachable!("Element will match .as_element()"),
                Selector::TextLike(_) => "text, tspan".to_string(),
                Selector::Class(_) => ".any-class".to_string(), // Placeholder for any class
                Selector::LineLike(_) => "line, polyline, path".to_string(),
                Selector::BasicShape(_) => "rect, circle, ellipse, polygon".to_string(),
            }
        } else {
            panic!("Cannot convert selector to CSS without a class or MatchResult");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub selector: Selector,
    pub styles: Vec<Style>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    Any,
    Element(String),
    Class(String),
    NumericSuffix(String, u32),
    ColourSuffix(String, String),
}

impl MatchResult {
    pub fn as_class(&self) -> Option<String> {
        match self {
            MatchResult::Any => None,
            MatchResult::Element(_) => None,
            MatchResult::Class(name) => Some(name.clone()),
            MatchResult::NumericSuffix(name, num) => Some(format!("{name}-{num}")),
            MatchResult::ColourSuffix(name, colour) => Some(format!("{name}-{colour}")),
        }
    }

    pub fn as_element(&self) -> Option<String> {
        if let MatchResult::Element(name) = self {
            Some(name.clone())
        } else {
            None
        }
    }

    pub fn colour(&self) -> Option<String> {
        if let MatchResult::ColourSuffix(_, colour) = self {
            Some(colour.clone())
        } else {
            None
        }
    }

    pub fn number(&self) -> Option<u32> {
        if let MatchResult::NumericSuffix(_, num) = self {
            Some(*num)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchType {
    Any,
    // TODO: Cow? Most cases here could use 'static str, but there are
    // just enough computed cases that we can't use that.
    Element(String),
    Class(String),
    NumericSuffix(String),
    ColourSuffix(String),
}

impl MatchType {
    fn matches_class(&self, class: &str) -> Option<MatchResult> {
        match self {
            MatchType::Any => Some(MatchResult::Any),
            MatchType::Element(_) => None,
            MatchType::Class(name) => {
                if class == *name {
                    Some(MatchResult::Class(class.to_string()))
                } else {
                    None
                }
            }
            MatchType::NumericSuffix(name) => {
                if let Some((prefix, suffix)) = class.rsplit_once('-') {
                    if prefix == *name {
                        if let Ok(num) = suffix.parse::<u32>() {
                            return Some(MatchResult::NumericSuffix(prefix.to_string(), num));
                        }
                    }
                }
                None
            }
            MatchType::ColourSuffix(name) => {
                if let Some((prefix, suffix)) = class.rsplit_once('-') {
                    if prefix == *name && COLOUR_LIST.binary_search(&suffix).is_ok() {
                        return Some(MatchResult::ColourSuffix(
                            prefix.to_string(),
                            suffix.to_string(),
                        ));
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_type_matches() {
        let mt = MatchType::Class("d-grid".to_string());
        assert_eq!(
            mt.matches_class("d-grid"),
            Some(MatchResult::Class("d-grid".to_string()))
        );
        assert_eq!(mt.matches_class("d-grid-5"), None);
        assert_eq!(mt.matches_class("d-red"), None);

        let mt = MatchType::NumericSuffix("d-grid".to_string());
        assert_eq!(
            mt.matches_class("d-grid-5"),
            Some(MatchResult::NumericSuffix("d-grid".to_string(), 5))
        );
        assert_eq!(mt.matches_class("d-grid"), None);
        assert_eq!(mt.matches_class("d-red"), None);

        let mt = MatchType::ColourSuffix("d".to_string());
        assert_eq!(
            mt.matches_class("d-red"),
            Some(MatchResult::ColourSuffix(
                "d".to_string(),
                "red".to_string()
            ))
        );
        assert_eq!(mt.matches_class("d-grid"), None);
        assert_eq!(mt.matches_class("d-grid-5"), None);
    }
}
