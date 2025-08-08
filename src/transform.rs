use crate::context::TransformerContext;
use crate::elements::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::events::{tagify_events, InputList, OutputEvent, OutputList, Tag};
use crate::geometry::{BoundingBox, BoundingBoxBuilder, LocSpec};
use crate::style::{self, ContextTheme};
use crate::types::{fstr, split_unit, AttrMap, OrderIndex};
use crate::{AutoStyleMode, TransformConfig};

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
                    events.push(OutputEvent::Text(tail.to_owned()));
                }
                // NOTE: el.content_bbox may be set (e.g. if symbol) while bb is None here.
            }
            Tag::Leaf(el, tail) => {
                let mut el = el.clone();
                context.apply_defaults(&mut el);
                let (ev, bb) = el.generate_events(context)?;
                (events, bbox) = (ev, bb);
                if let (Some(tail), false) = (tail, events.is_empty()) {
                    events.push(OutputEvent::Text(tail.to_owned()));
                }
            }
            Tag::Comment(_, c, tail) => {
                events.push(OutputEvent::Comment(c.clone()));
                if let Some(tail) = tail {
                    events.push(OutputEvent::Text(tail.to_owned()));
                }
            }
            Tag::Text(_, t) => {
                events.push(OutputEvent::Text(t.clone()));
            }
            Tag::CData(_, c) => {
                events.push(OutputEvent::CData(c.clone()));
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
    let mut element_errors: HashMap<OrderIndex, (SvgElement, SvgdxError)> = HashMap::new();
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
                        if let SvgdxError::MultiError(err_list) = err {
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
            return Err(SvgdxError::MultiError(element_errors));
        }

        mem::swap(tags, remain);
        remain.clear();
    }
    Ok(bbb.clone().build())
}

pub fn process_events(
    input: impl Into<InputList>,
    context: &mut TransformerContext,
) -> Result<(OutputList, Option<BoundingBox>)> {
    let input = input.into();
    if is_real_svg(&input) {
        if context.get_top_element().is_none() {
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

    fn make_root_svg(
        &self,
        first_svg: OutputEvent,
        bbox: Option<BoundingBox>,
    ) -> Result<SvgElement> {
        let mut new_svg_attrs = AttrMap::new();
        let mut orig_svg_attrs = HashMap::new();
        if let OutputEvent::Start(orig_svg) = first_svg {
            for (k, v) in orig_svg.get_attrs() {
                new_svg_attrs.insert(k, v);
            }
            orig_svg_attrs = orig_svg.get_attrs().into_iter().collect();
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
        if let Some(svg_style) = &self.context.config.svg_style {
            new_svg_attrs.insert("style", svg_style.as_str());
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
                let (x1, y1) = bb
                    .locspec(LocSpec::TopLeft)
                    .expect("using non lineoffset garenteed not to be none");
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
                    .filter_map(|output_ev| match output_ev {
                        OutputEvent::Start(e) | OutputEvent::Empty(e) => Some(e),
                        _ => None,
                    })
                    .collect();
                registry.process_inline(&mut elements);
            }
            AutoStyleMode::Css => {
                let elements: Vec<_> = events
                    .iter()
                    .filter_map(|output_ev| match output_ev {
                        OutputEvent::Start(e) | OutputEvent::Empty(e) => Some(e),
                        _ => None,
                    })
                    .collect();
                registry.process_css(&elements);
            }
        }
        registry.get_state()
    }

    fn autostyle_defs_events(&self, auto_defs: Vec<String>) -> Result<OutputList> {
        let indent = 2;
        let indent_line = |n| format!("\n{}", " ".repeat(n));

        if !auto_defs.is_empty() {
            let mut defs_events = vec![
                OutputEvent::Text(indent_line(indent)),
                OutputEvent::Start(SvgElement::new("defs", &[])),
            ];
            if self.context.config.debug {
                defs_events.extend([
                    OutputEvent::Text(indent_line(indent + 2)),
                    OutputEvent::Comment(" svgdx-generated auto-style defs ".to_owned()),
                ]);
            }
            defs_events.push(OutputEvent::Text("\n".to_owned()));
            let eee = InputList::from_str(&indent_all(auto_defs, indent + 2).join("\n"))?;
            defs_events.extend(OutputList::from(eee));
            defs_events.extend(vec![
                OutputEvent::Text(indent_line(indent)),
                OutputEvent::End("defs".to_owned()),
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
                OutputEvent::Text(indent_line(indent)),
                OutputEvent::Start(SvgElement::new("style", &[])),
            ];
            if self.context.config.debug {
                style_events.extend([
                    OutputEvent::Text(indent_line(indent + 2)),
                    OutputEvent::Comment(" svgdx-generated auto-style CSS ".to_owned()),
                ]);
            }
            style_events.extend(vec![
                OutputEvent::Text(indent_line(indent + 2)),
                OutputEvent::CData(format!(
                    "\n{}\n{}",
                    indent_all(auto_styles, indent + 4).join("\n"),
                    " ".repeat(indent + 2)
                )),
                OutputEvent::Text(indent_line(indent)),
                OutputEvent::End("style".to_owned()),
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
            let root_svg = self.make_root_svg(first_svg, bbox)?;
            output_events.push(OutputEvent::Start(root_svg));
            events = remain;
            has_svg_element = true;
        }

        if self.context.config.debug {
            let indent = "\n  ".to_owned();

            output_events.extend(
                vec![
                    OutputEvent::Text(indent.clone()),
                    OutputEvent::Comment(format!(
                        " Generated by {} v{} ",
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION")
                    )),
                    OutputEvent::Text(indent),
                    OutputEvent::Comment(format!(" Config: {:?} ", self.context.config)),
                ]
                .as_slice(),
            )
        }

        output_events.push(OutputEvent::Empty(SvgElement::new("style_sentinel", &[])));
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
