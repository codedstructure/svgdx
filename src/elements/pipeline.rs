use crate::constants::RELPOS_SEP;
use crate::context::TransformerContext;
use crate::elements::SvgElement;
use crate::elements::containers::is_graphics_element;
use crate::errors::Result;

pub fn element_pipeline(el: SvgElement, context: &mut TransformerContext) -> Result<SvgElement> {
    let next = apply_defaults(el, context);
    let next = inner_text_to_attr(next, context);
    let next = apply_auto_id(next, context);
    let next = convert_dirspec(next);

    Ok(next)
}

/// Apply the current default scope to an element without mutating the caller's input.
fn apply_defaults(el: SvgElement, context: &mut TransformerContext) -> SvgElement {
    let mut next = el;
    context.apply_defaults(&mut next);
    next
}

/// Convert text-only inner content on a graphics element into a `text` attribute
fn inner_text_to_attr(el: SvgElement, context: &TransformerContext) -> SvgElement {
    let mut next = el;
    if next.has_attr("text") || next.has_attr("md") {
        return next;
    }
    // Avoid coercing e.g. <description> or <title> elements.
    if !is_graphics_element(&next) {
        return next;
    }

    let Some(inner_events) = next.inner_events(context) else {
        return next;
    };

    let mut inner_text = None;
    for ev in inner_events.iter() {
        if let Some(text) = ev.text_string() {
            if inner_text.is_none() {
                inner_text = Some(text);
            }
        } else if let Some(cdata) = ev.cdata_string() {
            inner_text = Some(cdata);
        } else {
            return next;
        }
    }

    if let Some(text) = inner_text {
        // convert to an empty element with text attribute
        next.set_attr("text", &text);
        if let Some((start, _end)) = next.event_range {
            next.event_range = Some((start, start));
        }
    }
    next
}

/// Add a generated `id` based on a text-derived label when the element has .d-auto-id
fn apply_auto_id(el: SvgElement, context: &mut TransformerContext) -> SvgElement {
    let mut next = el;
    if !next.pop_class("d-auto-id") || next.has_attr("id") {
        return next;
    }

    // id is derived from text or markdown attributes if present
    let Some(text) = next.get_attr("text").or_else(|| next.get_attr("md")) else {
        return next;
    };

    let Some(candidate) = auto_id_from_text(text) else {
        return next;
    };

    // TODO: warn or append suffix if ID already exists
    if context.get_element_by_id(&candidate).is_none() {
        next.set_attr("id", &candidate);
    }
    next
}

// Convert xy with a dirspec (e.g. #abc|h) to relpos attr
fn convert_dirspec(el: SvgElement) -> SvgElement {
    let mut el = el;
    if el.get_attr("xy").unwrap_or_default().contains(RELPOS_SEP)
        && let Some(relpos) = el.pop_attr("xy")
    {
        el.set_attr("relpos", &relpos);
    }
    el
}

fn auto_id_from_text(text: &str) -> Option<String> {
    // split into non-empty alphanumeric words
    let mut words = text
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty());

    // convert to camelCase
    let mut auto_id = String::with_capacity(text.len());
    auto_id.push_str(&words.next()?.to_lowercase());
    for word in words {
        auto_id.push_str(&word[..1].to_uppercase());
        auto_id.push_str(&word[1..].to_lowercase());
    }
    Some(auto_id)
}

#[cfg(test)]
mod tests {
    use super::auto_id_from_text;

    #[test]
    fn test_auto_id_from_text() {
        assert_eq!(
            auto_id_from_text("Hello World"),
            Some("helloWorld".to_string())
        );
        assert_eq!(
            auto_id_from_text("  multiple   spaces "),
            Some("multipleSpaces".to_string())
        );
        assert_eq!(auto_id_from_text(""), None);
        assert_eq!(
            auto_id_from_text("123 numbers"),
            Some("123Numbers".to_string())
        );
        assert_eq!(
            auto_id_from_text("$symbols&punctuation-?."),
            Some("symbolsPunctuation".to_string())
        );
    }
}
