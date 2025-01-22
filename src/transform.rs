use crate::context::{ElementMap, TransformerContext};
use crate::element::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::events::{tagify_events, InputList, OutputEvent, OutputList, Tag};
use crate::expression::{eval_attr, eval_condition};
use crate::loop_el::{ForElement, LoopElement};
use crate::position::{BoundingBox, BoundingBoxBuilder, LocSpec};
use crate::reuse::ReuseElement;
use crate::themes::ThemeBuilder;
use crate::types::{fstr, split_unit, AttrMap, OrderIndex};
use crate::TransformConfig;

use std::collections::{BTreeMap, HashMap, HashSet};
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

impl EventGen for SvgElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        context.inc_depth()?;
        let res = match self.name.as_str() {
            "loop" => LoopElement(self.clone()).generate_events(context),
            "config" => ConfigElement(self.clone()).generate_events(context),
            "reuse" => ReuseElement(self.clone()).generate_events(context),
            "specs" => SpecsElement(self.clone()).generate_events(context),
            "var" => VarElement(self.clone()).generate_events(context),
            "if" => IfElement(self.clone()).generate_events(context),
            "defaults" => DefaultsElement(self.clone()).generate_events(context),
            "for" => ForElement(self.clone()).generate_events(context),
            "g" | "symbol" => GroupElement(self.clone()).generate_events(context),
            _ => {
                if let Some((start, end)) = self.event_range {
                    if start != end {
                        return Container(self.clone()).generate_events(context);
                    }
                }
                OtherElement(self.clone()).generate_events(context)
            }
        };
        context.dec_depth()?;
        res
    }
}

#[derive(Debug, Clone)]
struct DefaultsElement(SvgElement);

impl EventGen for DefaultsElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        for ev in self.0.inner_events(context).unwrap_or_default() {
            // we only care about Element-generating (i.e. start/empty) events
            if let Ok(el) = SvgElement::try_from(ev.clone()) {
                context.set_element_default(&el);
            }
        }
        Ok((OutputList::new(), None))
    }
}

/// Container will be used for many elements which contain other elements,
/// but have no independent behaviour, such as defs, linearGradient, etc.
#[derive(Debug, Clone)]
struct Container(SvgElement);

impl EventGen for Container {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        if let Some(inner_events) = self.0.inner_events(context) {
            // If there's only text/cdata events, apply to current element and render
            let mut inner_text = None;
            for e in inner_events.iter() {
                if let Some(t) = e.text_string() {
                    if inner_text.is_none() {
                        inner_text = Some(t);
                    }
                } else if let Some(c) = e.cdata_string() {
                    inner_text = Some(c);
                } else {
                    // not text or cdata - abandon the effort and mark as such.
                    inner_text = None;
                    break;
                }
            }
            if let (true, Some(text)) = (self.0.is_graphics_element(), &inner_text) {
                let mut el = self.0.clone();
                el.set_attr("text", text);
                if let Some((start, _end)) = self.0.event_range {
                    el.event_range = Some((start, start)); // emulate an Empty element
                }
                el.generate_events(context)
            } else {
                let mut new_el = self.0.clone();
                // Special case <svg> elements with an xmlns attribute - passed through
                // transparently, with no bbox calculation.
                if new_el.name == "svg" && new_el.get_attr("xmlns").is_some() {
                    return Ok((self.0.all_events(context).into(), None));
                }
                new_el.eval_attributes(context);
                if context.config.add_metadata {
                    new_el
                        .attrs
                        .insert("data-src-line", self.0.src_line.to_string());
                }
                let mut events = OutputList::new();
                events.push(OutputEvent::Start(new_el));
                let (evlist, mut bbox) = if inner_text.is_some() {
                    // inner_text implies no processable events; use as-is
                    (inner_events.into(), None)
                } else {
                    process_events(inner_events, context)?
                };
                events.extend(&evlist);
                events.push(OutputEvent::End(self.0.name.clone()));

                if self.0.name == "defs" || self.0.name == "symbol" {
                    bbox = None;
                }

                Ok((events, bbox))
            }
        } else {
            Ok((OutputList::new(), None))
        }
    }
}

#[derive(Debug, Clone)]
struct OtherElement(SvgElement);

impl EventGen for OtherElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut output = OutputList::new();
        let mut e = self.0.clone();
        e.resolve_position(context)?; // transmute assumes some of this (e.g. dxy -> dx/dy) has been done
        e.transmute(context)?;
        e.resolve_position(context)?;
        context.update_element(&e);
        let mut bb = context.get_element_bbox(&e)?;
        if bb.is_some() {
            context.set_prev_element(&e);
        }
        let events = e.element_events(context)?;
        for svg_ev in events {
            let is_empty = matches!(svg_ev, OutputEvent::Empty(_));
            let adapted = if let OutputEvent::Empty(e) | OutputEvent::Start(e) = svg_ev {
                let mut new_el = SvgElement::new(&e.name, &[]);
                // Collect pass-through attributes
                for (k, v) in e.attrs {
                    if k != "class" && k != "data-src-line" && k != "_" && k != "__" {
                        new_el.set_attr(&k, &v);
                    }
                }
                // Any 'class' attribute values are stored separately as a HashSet;
                // collect those into the BytesStart object
                if !e.classes.is_empty() {
                    new_el.add_classes(&e.classes);
                }
                // Add 'data-src-line' for all elements generated by input `element`
                if context.config.add_metadata {
                    new_el.set_attr("data-src-line", &e.src_line.to_string());
                }
                if is_empty {
                    OutputEvent::Empty(new_el)
                } else {
                    OutputEvent::Start(new_el)
                }
            } else {
                svg_ev
            };

            output.push(adapted);
        }
        if self.0.name == "point" {
            // point elements have no bounding box, and are primarily used for
            // update_element() side-effects, e.g. setting prev_element.
            // (They can generate text though, so not rejected earlier.
            bb = None;
        }
        Ok((output, bb))
    }
}

#[derive(Debug, Clone)]
struct GroupElement(SvgElement);

impl EventGen for GroupElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        // since we synthesize the opening element event here, we need to
        // do any required transformations on the <g> itself here.
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context);

        // push variables onto the stack
        context.push_element(&self.0);

        let mut content_bb = None;
        let mut events = OutputList::new();
        if self.0.is_empty_element() {
            events.push(OutputEvent::Empty(new_el));
        } else {
            let el_name = new_el.name.clone();
            events.push(OutputEvent::Start(new_el));

            if let Some(inner_events) = self.0.inner_events(context) {
                let (ev_list, bb) = process_events(inner_events, context)?;
                content_bb = bb;
                events.extend(&ev_list);
            }

            events.push(OutputEvent::End(el_name));
        }

        // pop variables off the stack
        context.pop_element();

        // Messy! should probably have a id->bbox map in context
        let mut new_el = self.0.clone();
        new_el.content_bbox = content_bb;
        context.update_element(&new_el);
        context.set_prev_element(&new_el);

        let result_bb = if self.0.name == "symbol" {
            // symbols have a size which needs storing in context for evaluating
            // bbox of 'use' elements referencing them, but they don't contribute
            // to the parent bbox.
            None
        } else {
            // this handles any `transform` attr. Assumes .content_bbox is set.
            new_el.bbox()?
        };
        Ok((events, result_bb))
    }
}

#[derive(Debug, Clone)]
struct ConfigElement(SvgElement);

impl EventGen for ConfigElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut new_config = context.config.clone();
        for (key, value) in &self.0.attrs {
            match key.as_str() {
                "scale" => new_config.scale = value.parse()?,
                "debug" => new_config.debug = value.parse()?,
                "add-auto-styles" => new_config.add_auto_styles = value.parse()?,
                "use-local-styles" => new_config.use_local_styles = value.parse()?,
                "border" => new_config.border = value.parse()?,
                "background" => new_config.background.clone_from(value),
                "loop-limit" => new_config.loop_limit = value.parse()?,
                "var-limit" => new_config.var_limit = value.parse()?,
                "depth-limit" => new_config.depth_limit = value.parse()?,
                "font-size" => new_config.font_size = value.parse()?,
                "font-family" => new_config.font_family.clone_from(value),
                "seed" => new_config.seed = value.parse()?,
                "theme" => new_config.theme = value.parse()?,
                "svg-style" => new_config.svg_style = Some(value.clone()),
                _ => {
                    return Err(SvgdxError::InvalidData(format!(
                        "Unknown config setting {key}"
                    )))
                }
            }
        }
        context.set_config(new_config);
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
struct SpecsElement(SvgElement);

impl EventGen for SpecsElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        if context.in_specs {
            return Err(SvgdxError::DocumentError(
                "Nested <specs> elements are not allowed".to_string(),
            ));
        }
        if let Some(inner_events) = self.0.inner_events(context) {
            context.in_specs = true;
            process_events(inner_events, context)?;
            context.in_specs = false;
        }
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
struct VarElement(SvgElement);

impl EventGen for VarElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        // variables are updated 'in parallel' rather than one-by-one,
        // allowing e.g. swap in a single `<var>` element:
        // `<var a="$b" b="$a" />`
        let mut new_vars = Vec::new();
        for (key, value) in self.0.attrs.clone() {
            // Note comments in `var` elements are permitted (and encouraged!)
            // in the input, but not propagated to the output.
            if key != "_" && key != "__" {
                let value = eval_attr(&value, context);
                // Detect / prevent uncontrolled expansion of variable values
                if value.len() > context.config.var_limit as usize {
                    return Err(SvgdxError::VarLimitError(
                        key.clone(),
                        value.len(),
                        context.config.var_limit,
                    ));
                }
                new_vars.push((key, value));
            }
        }
        for (k, v) in new_vars.into_iter() {
            context.set_var(&k, &v);
        }
        Ok((OutputList::new(), None))
    }
}

#[derive(Debug, Clone)]
struct IfElement(SvgElement);

impl EventGen for IfElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let test = self
            .0
            .get_attr("test")
            .ok_or_else(|| SvgdxError::MissingAttribute("test".to_owned()))?;
        if let Some(inner_events) = self.0.inner_events(context) {
            if eval_condition(&test, context)? {
                // opening if element is not included in the processed inner events to avoid
                // infinite recursion...
                return process_events(inner_events.clone(), context);
            }
        }

        Ok((OutputList::new(), None))
    }
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
            if el.name == "svg" {
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
            Tag::Comment(c, tail) => {
                events.push(OutputEvent::Comment(c.clone()));
                if let Some(tail) = tail {
                    events.push(OutputEvent::Text(tail.to_owned()));
                }
            }
            Tag::Text(t) => {
                events.push(OutputEvent::Text(t.clone()));
            }
            Tag::CData(c) => {
                events.push(OutputEvent::CData(c.clone()));
            }
        }
        Ok((events, bbox))
    }
}

fn process_tags(
    tags: &mut Vec<(OrderIndex, Tag)>,
    context: &mut TransformerContext,
    idx_output: &mut BTreeMap<OrderIndex, OutputList>,
    bbb: &mut BoundingBoxBuilder,
) -> Result<Option<BoundingBox>> {
    let mut element_errors: HashMap<OrderIndex, (SvgElement, SvgdxError)> = HashMap::new();
    let remain = &mut Vec::new();

    while !tags.is_empty() && remain.len() != tags.len() {
        for (idx, t) in &mut tags.iter_mut() {
            let idx = idx.clone();
            let el = if let Some(el) = t.get_element() {
                // update early so reuse targets are available even if the element
                // is not ready (e.g. within a specs block)
                context.update_element(&el);
                Some(el.clone())
            } else {
                None
            };
            let gen_result = t.generate_events(context);
            if !context.in_specs {
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
                    remain.push((idx, t.clone()));
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
    input: InputList,
    context: &mut TransformerContext,
) -> Result<(OutputList, Option<BoundingBox>)> {
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
    let mut tags = tagify_events(input)?
        .iter()
        .enumerate()
        .map(|(idx, el)| (OrderIndex::new(idx), el.clone()))
        .collect::<Vec<_>>();
    let bbox = process_tags(&mut tags, context, &mut idx_output, &mut bbb)?;

    for (_idx, events) in idx_output {
        output.extend(&events);
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

    fn write_root_svg(
        &self,
        first_svg: OutputEvent,
        bbox: Option<BoundingBox>,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let mut new_svg_attrs = AttrMap::new();
        let mut orig_svg_attrs = HashMap::new();
        if let OutputEvent::Start(orig_svg) = first_svg {
            new_svg_attrs = orig_svg.attrs.clone();
            orig_svg_attrs = orig_svg.get_attrs();
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
                let new_width = format!("{}mm", width);
                let new_height = format!("{}mm", height);
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

        OutputList::from(
            [OutputEvent::Start(SvgElement::new(
                "svg",
                &new_svg_attrs.to_vec(),
            ))]
            .as_slice(),
        )
        .write_to(writer)
    }

    fn write_auto_styles(&self, events: &mut OutputList, writer: &mut dyn Write) -> Result<()> {
        // Collect the set of elements and classes so relevant styles can be
        // automatically added.
        let mut element_set = HashSet::new();
        let mut class_set = HashSet::new();
        for output_ev in events.iter() {
            match output_ev {
                OutputEvent::Start(e) | OutputEvent::Empty(e) => {
                    element_set.insert(e.name.clone());
                    class_set.extend(e.get_classes());
                }
                _ => {}
            }
        }

        let indent = 2;
        let mut tb = ThemeBuilder::new(&self.context, &element_set, &class_set);
        tb.build();
        let auto_defs = tb.get_defs();
        let auto_styles = tb.get_styles();

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
            OutputList::from(defs_events).write_to(writer)?;
        }
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
            OutputList::from(style_events).write_to(writer)?;
        }
        Ok(())
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

        let mut has_svg_element = false;
        if let (pre_svg, Some(first_svg), remain) = events.partition("svg") {
            pre_svg.write_to(writer)?;
            self.write_root_svg(first_svg, bbox, writer)?;
            events = remain;
            has_svg_element = true;
        }

        if self.context.config.debug {
            let indent = "\n  ".to_owned();

            OutputList::from(vec![
                OutputEvent::Text(indent.clone()),
                OutputEvent::Comment(format!(
                    " Generated by {} v{} ",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                )),
                OutputEvent::Text(indent),
                OutputEvent::Comment(format!(" Config: {:?} ", self.context.config)),
            ])
            .write_to(writer)?;
        }

        // Default behaviour: include auto defs/styles iff we have an SVG element,
        // i.e. this is a full SVG document rather than a fragment.
        if has_svg_element && self.context.config.add_auto_styles {
            self.write_auto_styles(&mut events, writer)?;
        }

        events.write_to(writer)
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
            assert!(is_real_svg(&input.parse().unwrap()), "{:?}", input);
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
            assert!(!is_real_svg(&input.parse().unwrap()), "{:?}", input);
        }
    }

    #[test]
    fn test_process_seq() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let seq = InputList::new();

        process_events(seq, &mut transformer.context).unwrap();
    }

    #[test]
    fn test_process_tags_multiple_elements() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let mut idx_output = BTreeMap::new();

        let seq = InputList::from_str(
            r##"<svg>
          <rect xy="#a:h" wh="10"/>
          <circle id="a" cx="50" cy="50" r="40"/>
        </svg>"##,
        )
        .unwrap();

        transformer.context.set_events(seq.events.clone());
        let mut tags = tagify_events(seq)
            .unwrap()
            .iter()
            .enumerate()
            .map(|(idx, el)| (OrderIndex::new(idx), el.clone()))
            .collect::<Vec<_>>();
        let bbb = &mut BoundingBoxBuilder::new();

        let result = process_tags(&mut tags, &mut transformer.context, &mut idx_output, bbb);
        assert!(result.is_ok());

        // let ok_ev_count = idx_output
        //     .iter()
        //     .map(|entry| entry.1.events.len())
        //     .reduce(|a, b| a + b)
        //     .unwrap();
        // assert_eq!(ok_ev_count, 7);
    }

    #[test]
    fn test_indent_all() {
        let input = vec!["a".to_string(), "  b".to_string(), "c".to_string()];
        let output = indent_all(input, 2);
        assert_eq!(output, vec!["  a", "    b", "  c"]);
    }
}
