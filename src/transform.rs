use crate::context::TransformerContext;
use crate::document::tag::{tagify_events, Tag};
use crate::document::{EventKind, EventStyleWrapper, InputList, OutputList};
use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, BoundingBoxBuilder, LocSpec};
use crate::style::{self, ContextTheme};
use crate::types::{fstr, split_unit, AttrMap, OrderIndex};
use crate::{AutoStyleMode, ErrorMode, TransformConfig};

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, Write};
use std::mem;
use std::str::FromStr;

pub trait EventGen {
    /// Determine the sequence of (XML-level) events to emit in response
    /// to a given item, as well as the corresponding bounding box.
    ///
    /// Note some implementations may mutate the context (e.g. `var` elements).
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)>;
}

/// Check if the input events represent a "real" SVG document
///
/// This is determined by checking for the first Start event being `<svg>`
/// with a valid SVG 'xmlns' attribute. Note there may be *events* such as
/// processing instructions or comments before the first Start event.
///
/// This does *not* check that the entire doc is valid, and is intended
/// to be fast in common cases.
fn is_real_svg(events: &InputList) -> bool {
    for ev in events.iter() {
        if let Ok(el) = SvgElement::try_from(ev.clone()) {
            // "Real" SVG documents will have an `xmlns` attribute with
            // the value "http://www.w3.org/2000/svg"
            if el.name() == "svg" {
                if let Some(val) = el.get_attr("xmlns") {
                    return val == "http://www.w3.org/2000/svg";
                }
            }
            return false;
        }
    }
    false
}

impl EventGen for Tag {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut events = OutputList::new();
        let mut bbox = None;
        match self {
            Tag::Compound(el, tail) => {
                let (ev, bb) = el.generate_events(context)?;
                (events, bbox) = (ev, bb);
                if let (Some(tail), false) = (tail, events.is_empty()) {
                    events.push(EventKind::Text(tail.to_owned()));
                }
                // NOTE: el.content_bbox may be set (e.g. if symbol) while bb is None here.
            }
            Tag::Leaf(el, tail) => {
                let mut el = el.clone();
                context.apply_defaults(&mut el);
                let (ev, bb) = el.generate_events(context)?;
                (events, bbox) = (ev, bb);
                if let (Some(tail), false) = (tail, events.is_empty()) {
                    events.push(EventKind::Text(tail.to_owned()));
                }
            }
            Tag::Comment(_, c, tail) => {
                events.push(EventKind::Comment(c.clone()));
                if let Some(tail) = tail {
                    events.push(EventKind::Text(tail.to_owned()));
                }
            }
            Tag::Text(_, t) => {
                events.push(EventKind::Text(t.clone()));
            }
            Tag::CData(_, c) => {
                events.push(EventKind::CData(c.clone()));
            }
        }
        Ok((events, bbox))
    }
}

fn process_tags(
    tags: &mut Vec<Tag>,
    context: &mut TransformerContext,
    idx_output: &mut BTreeMap<OrderIndex, OutputList>,
    bbb: &mut BoundingBoxBuilder,
) -> Result<Option<BoundingBox>> {
    let mut element_errors: HashMap<OrderIndex, (SvgElement, Error)> = HashMap::new();
    let remain = &mut Vec::new();

    while !tags.is_empty() && remain.len() != tags.len() {
        for t in &mut tags.iter_mut() {
            let el = t.get_element_mut().cloned();
            let gen_result = t.generate_events(context);
            if !context.in_specs {
                let idx = t.get_order_index();
                // if we *are* in a specs block, we don't care if there were errors;
                // a specs entry may have insufficient context until reuse time.
                // We do still call generate_events for side-effects including registering
                // elements for reuse.
                if let Ok((events, maybe_bbox)) = gen_result {
                    if let Some(bbox) = maybe_bbox {
                        bbb.extend(bbox); // TODO: should this pattern take an Option?
                    }
                    if !events.is_empty() {
                        idx_output.insert(idx, events);
                    }
                } else {
                    if let (Some(el), Err(err)) = (el, gen_result) {
                        if let Error::Multi(err_list) = err {
                            for (idx, (el, err)) in err_list {
                                element_errors.insert(idx, (el, err));
                            }
                        } else {
                            element_errors.insert(idx.clone(), (el, err));
                        }
                    }
                    remain.push(t.clone());
                }
            }
        }
        if tags.len() == remain.len() {
            // no progress made; abandon further processing
            break;
        }

        mem::swap(tags, remain);
        remain.clear();
        element_errors.clear();
    }

    if !element_errors.is_empty() {
        handle_errors(element_errors, context, idx_output)?;
    }
    Ok(bbb.clone().build())
}

pub fn handle_errors(
    element_errors: HashMap<OrderIndex, (SvgElement, Error)>,
    context: &TransformerContext,
    idx_output: &mut BTreeMap<OrderIndex, OutputList>,
) -> Result<()> {
    match context.config.error_mode {
        ErrorMode::Strict => Err(Error::Multi(element_errors)),
        ErrorMode::Warn => {
            for (idx, (el, err)) in element_errors {
                let mut ev_list = OutputList::from(vec![
                    EventKind::Text("\n".to_owned()),
                    EventKind::Comment(format!(" Warning: error processing element: {:?} ", err)),
                ]);
                let el_events = el.all_events(context);
                ev_list.extend(el_events);
                ev_list.push(EventKind::Text("\n".to_owned()));
                idx_output.insert(idx, ev_list);
            }
            Ok(())
        }
    }
}

pub fn process_events(
    input: impl Into<InputList>,
    context: &mut TransformerContext,
) -> Result<(OutputList, Option<BoundingBox>)> {
    let input = input.into();
    if is_real_svg(&input) {
        if context.is_top_level() {
            // if this is the outermost SVG element, we mark the entire input as a 'real' SVG document
            context.real_svg = true;
        }
        return Ok((input.into(), None));
    }
    let mut output = OutputList::new();
    let mut idx_output = BTreeMap::<OrderIndex, OutputList>::new();

    let mut bbb = BoundingBoxBuilder::new();
    let mut tags = tagify_events(input)?;
    let bbox = process_tags(&mut tags, context, &mut idx_output, &mut bbb)?;

    for (_idx, events) in idx_output {
        output.extend(events);
    }

    Ok((output, bbox))
}

pub struct Transformer {
    pub context: TransformerContext,
}

impl Transformer {
    pub fn from_config(config: &TransformConfig) -> Self {
        Self {
            context: TransformerContext::from_config(config),
        }
    }

    pub fn transform(&mut self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let input = InputList::from_reader(reader)?;
        self.context.set_events(input.events.clone());
        let output = process_events(input, &mut self.context)?;
        self.postprocess(output, writer)
    }

    fn make_root_svg(&self, first_svg: EventKind, bbox: Option<BoundingBox>) -> Result<SvgElement> {
        let mut new_svg_attrs = AttrMap::new();
        let mut orig_svg_attrs: HashMap<String, String> = HashMap::new();
        let mut orig_svg_class = None;
        let mut orig_svg_style = None;
        if let EventKind::Start(orig_svg) = first_svg {
            for (k, v) in orig_svg.get_attrs() {
                new_svg_attrs.insert(k, v);
            }

            let el = SvgElement::new(orig_svg.name(), &new_svg_attrs.to_vec());
            orig_svg_attrs = orig_svg.get_attrs().iter().cloned().collect();
            orig_svg_class = Some(el.get_classes());
            orig_svg_style = Some(el.get_styles().clone());
        }
        if !orig_svg_attrs.contains_key("version") {
            new_svg_attrs.insert("version", "1.1");
        }
        if !orig_svg_attrs.contains_key("xmlns") {
            new_svg_attrs.insert("xmlns", "http://www.w3.org/2000/svg");
        }
        if !orig_svg_attrs.contains_key("id") {
            if let Some(local_id) = &self.context.local_style_id {
                new_svg_attrs.insert("id", local_id.as_str());
            }
        }
        let mut root_style = orig_svg_style.map(|s| s.to_string()).unwrap_or_default();
        if let Some(svg_style) = &self.context.config.svg_style {
            root_style.push(' ');
            root_style.push_str(svg_style.as_str());
        }
        if !root_style.is_empty() {
            new_svg_attrs.insert("style", root_style);
        }
        if let Some(class) = orig_svg_class {
            if !class.is_empty() {
                new_svg_attrs.insert("class", class.join(" "));
            }
        }
        // If width or height are provided, leave width/height/viewBox alone.
        let orig_width = orig_svg_attrs.get("width");
        let orig_height = orig_svg_attrs.get("height");
        // Expand by given border width
        let mut extent = bbox;
        if let Some(bb) = &mut extent {
            bb.expand(
                self.context.config.border as f32,
                self.context.config.border as f32,
            );
            bb.round();

            let aspect_ratio = bb.width() / bb.height();
            let view_width = fstr(bb.width());
            let view_height = fstr(bb.height());

            // Populate any missing width/height attributes
            if orig_width.is_none() && orig_height.is_none() {
                // if neither present, assume user units are mm, scaled by config.scale
                let width = fstr(bb.width() * self.context.config.scale);
                let height = fstr(bb.height() * self.context.config.scale);
                let new_width = format!("{width}mm");
                let new_height = format!("{height}mm");
                new_svg_attrs.insert("width", new_width.as_str());
                new_svg_attrs.insert("height", new_height.as_str());
            } else if orig_height.is_none() {
                let (width, unit) = split_unit(orig_width.expect("logic"))?;
                let new_height = format!("{}{}", fstr(width / aspect_ratio), unit);
                new_svg_attrs.insert("height", new_height.as_str());
            } else if orig_width.is_none() {
                let (height, unit) = split_unit(orig_height.expect("logic"))?;
                let new_width = format!("{}{}", fstr(height * aspect_ratio), unit);
                new_svg_attrs.insert("width", new_width.as_str());
            }

            if !orig_svg_attrs.contains_key("viewBox") {
                let (x1, y1) = bb.locspec(LocSpec::TopLeft);
                new_svg_attrs.insert(
                    "viewBox",
                    format!("{} {} {} {}", fstr(x1), fstr(y1), view_width, view_height).as_str(),
                );
            }
        }

        Ok(SvgElement::new("svg", &new_svg_attrs.to_vec()))
    }

    fn build_auto_styles(&self, events: &mut OutputList) -> (Vec<String>, Vec<String>) {
        // Collect the set of elements and classes so relevant styles can be
        // automatically added.
        let theme = ContextTheme::from_context(&self.context);
        let mut registry = style::StyleRegistry::new(&theme);
        match self.context.config.auto_style_mode {
            AutoStyleMode::None => {}
            AutoStyleMode::Inline => {
                let mut elements: Vec<_> = events
                    .iter_mut()
                    .filter_map(|output_ev| EventStyleWrapper::from_event(&mut output_ev.event))
                    .collect();
                let mut element_refs: Vec<_> = elements.iter_mut().collect();
                registry.process_inline(&mut element_refs);
            }
            AutoStyleMode::Css => {
                // TODO: use similar EventStyleWrapper approach here?
                let elements: Vec<_> = events
                    .iter()
                    .filter_map(|output_ev| match &output_ev.event {
                        EventKind::Start(e) | EventKind::Empty(e) => {
                            let e = SvgElement::new(e.name(), e.get_attrs());
                            Some(e)
                        }
                        _ => None,
                    })
                    .collect();
                let element_refs: Vec<_> = elements.iter().collect();
                registry.process_css(&element_refs);
            }
        }
        registry.get_state()
    }

    fn autostyle_defs_events(&self, auto_defs: Vec<String>) -> Result<OutputList> {
        let indent = 2;
        let indent_line = |n| format!("\n{}", " ".repeat(n));

        if !auto_defs.is_empty() {
            let mut defs_events = vec![
                EventKind::Text(indent_line(indent)),
                EventKind::Start(SvgElement::new("defs", &[]).into()),
            ];
            if self.context.config.debug {
                defs_events.extend([
                    EventKind::Text(indent_line(indent + 2)),
                    EventKind::Comment(" svgdx-generated auto-style defs ".to_owned()),
                ]);
            }
            defs_events.push(EventKind::Text("\n".to_owned()));
            let eee = InputList::from_str(&indent_all(auto_defs, indent + 2).join("\n"))?;
            defs_events.extend(OutputList::from(eee));
            defs_events.extend(vec![
                EventKind::Text(indent_line(indent)),
                EventKind::End("defs".to_owned()),
            ]);
            Ok(OutputList::from(defs_events))
        } else {
            Ok(OutputList::new())
        }
    }

    fn autostyle_css_events(&self, auto_styles: Vec<String>) -> Result<OutputList> {
        let indent = 2;
        let indent_line = |n| format!("\n{}", " ".repeat(n));

        if !auto_styles.is_empty() {
            let mut style_events = vec![
                EventKind::Text(indent_line(indent)),
                EventKind::Start(SvgElement::new("style", &[]).into()),
            ];
            if self.context.config.debug {
                style_events.extend([
                    EventKind::Text(indent_line(indent + 2)),
                    EventKind::Comment(" svgdx-generated auto-style CSS ".to_owned()),
                ]);
            }
            style_events.extend(vec![
                EventKind::Text(indent_line(indent + 2)),
                EventKind::CData(format!(
                    "\n{}\n{}",
                    indent_all(auto_styles, indent + 4).join("\n"),
                    " ".repeat(indent + 2)
                )),
                EventKind::Text(indent_line(indent)),
                EventKind::End("style".to_owned()),
            ]);
            Ok(OutputList::from(style_events))
        } else {
            Ok(OutputList::new())
        }
    }

    fn postprocess(
        &self,
        output: (OutputList, Option<BoundingBox>),
        writer: &mut dyn Write,
    ) -> Result<()> {
        let (mut events, bbox) = output;

        if self.context.real_svg {
            // We don't do any post-processing on 'real' SVG documents
            return events.write_to(writer);
        }

        let mut output_events = OutputList::new();

        let mut has_svg_element = false;
        if let (pre_svg, Some(first_svg), remain) = events.partition("svg") {
            output_events.extend(pre_svg);
            let root_svg = self.make_root_svg(first_svg.event, bbox)?;
            output_events.push(EventKind::Start(root_svg.into()));
            events = remain;
            has_svg_element = true;
        }

        if self.context.config.debug {
            let indent = "\n  ".to_owned();

            output_events.extend(vec![
                EventKind::Text(indent.clone()),
                EventKind::Comment(format!(
                    " Generated by {} v{} ",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                )),
                EventKind::Text(indent),
                EventKind::Comment(format!(" Config: {:?} ", self.context.config)),
            ])
        }

        output_events.push(EventKind::Empty(
            SvgElement::new("style_sentinel", &[]).into(),
        ));
        output_events.extend(events);

        // Default behaviour: include auto defs/styles iff we have an SVG element,
        // i.e. this is a full SVG document rather than a fragment.
        let mut style_events = OutputList::new();
        if has_svg_element {
            let (styles, defs) = self.build_auto_styles(&mut output_events);
            style_events.extend(self.autostyle_defs_events(defs)?);
            style_events.extend(self.autostyle_css_events(styles)?);
        }

        let (pre_style, _sentinel, post_style) = output_events.partition("style_sentinel");

        pre_style.write_to(writer)?;
        style_events.write_to(writer)?;
        post_style.write_to(writer)?;

        Ok(())
    }
}

// Helper function to indent all lines in a vector of strings
fn indent_all(s: Vec<String>, indent: usize) -> Vec<String> {
    let mut result = vec![];
    for entry in s {
        let mut rs = String::new();
        for (idx, line) in entry.lines().enumerate() {
            if idx > 0 {
                rs.push('\n');
            }
            rs.push_str(&" ".repeat(indent).to_owned());
            rs.push_str(line);
        }
        result.push(rs);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_real_svg() {
        let real_inputs = [
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <svg xmlns="http://www.w3.org/2000/svg">
                <rect width="100" height="100" />
            </svg>"#,
            r#"<!-- Comment!! -->
            <!-- Another comment -->
            <svg xmlns="http://www.w3.org/2000/svg">
                <rect width="100" height="100" />
            </svg>"#,
            r#"<svg xmlns="http://www.w3.org/2000/svg">
                <rect width="100" height="100" />
            </svg>"#,
        ];
        for input in real_inputs {
            assert!(is_real_svg(&input.parse().unwrap()), "{input:?}");
        }

        let unreal_inputs = [
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <svg>
                <rect width="100" height="100" />
            </svg>"#,
            r#"<svg>
                <rect width="100" height="100" />
            </svg>"#,
            r#"<!-- Comment!! -->
            <!-- Not 'real SVG' - has a non-svg first element -->
            <rect width="100" height="100"/>
            <svg xmlns="http://www.w3.org/2000/svg">
                <rect width="100" height="100" />
            </svg>"#,
            r#"<rect width="100" height="100"/>"#,
        ];
        for input in unreal_inputs {
            assert!(!is_real_svg(&input.parse().unwrap()), "{input:?}");
        }
    }

    #[test]
    fn test_process_seq() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let seq = InputList::new();

        process_events(seq, &mut transformer.context).unwrap();
    }

    #[test]
    fn test_indent_all() {
        let input = vec!["a".to_string(), "  b".to_string(), "c".to_string()];
        let output = indent_all(input, 2);
        assert_eq!(output, vec!["  a", "    b", "  c"]);
    }
}
