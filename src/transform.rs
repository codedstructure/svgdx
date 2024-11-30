use crate::context::{ElementMap, TransformerContext};
use crate::element::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::events::{EventList, InputEvent, SvgEvent};
use crate::expression::{eval_attr, eval_condition};
use crate::loop_el::LoopElement;
use crate::position::{BoundingBox, BoundingBoxBuilder, LocSpec};
use crate::reuse::ReuseElement;
use crate::themes::ThemeBuilder;
use crate::types::{fstr, split_unit, OrderIndex};
use crate::TransformConfig;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};
use std::mem;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};

pub trait EventGen {
    /// Determine the sequence of (XML-level) events to emit in response
    /// to a given item, as well as the corresponding bounding box.
    ///
    /// Note some implementations may mutate the context (e.g. `var` elements).
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)>;
}

impl EventGen for SvgElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        match self.name.as_str() {
            "loop" => LoopElement(self.clone()).generate_events(context),
            "config" => ConfigElement(self.clone()).generate_events(context),
            "reuse" => ReuseElement(self.clone()).generate_events(context),
            "specs" => SpecsElement(self.clone()).generate_events(context),
            "var" => VarElement(self.clone()).generate_events(context),
            "if" => IfElement(self.clone()).generate_events(context),
            "g" => GroupElement(self.clone()).generate_events(context),
            _ => {
                if let Some((start, end)) = self.event_range {
                    if start != end {
                        return Container(self.clone()).generate_events(context);
                    }
                }
                OtherElement(self.clone()).generate_events(context)
            }
        }
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
    ) -> Result<(EventList, Option<BoundingBox>)> {
        if let Some((start, end)) = self.0.event_range {
            let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);
            // If there's only text/cdata events, apply to current element and render
            let mut inner_text = None;
            for e in inner_events.iter() {
                match &e.event {
                    Event::Text(t) => {
                        if inner_text.is_none() {
                            inner_text = Some(String::from_utf8(t.to_vec())?)
                        }
                    }
                    Event::CData(c) => inner_text = Some(String::from_utf8(c.to_vec())?),
                    _ => {
                        // abandon the effort and mark as such.
                        inner_text = None;
                        break;
                    }
                }
            }
            if let (true, Some(text)) = (self.0.is_graphics_element(), &inner_text) {
                let mut el = self.0.clone();
                el.set_attr("text", text);
                el.event_range = Some((start, start)); // emulate an Empty element
                el.generate_events(context)
            } else {
                let mut new_el = self.0.clone();
                new_el.eval_attributes(context);
                let mut events = EventList::new();
                events.push(SvgEvent::Start(new_el));
                let (evlist, mut bbox) = if inner_text.is_some() {
                    // inner_text implies no processable events; use as-is
                    (inner_events, None)
                } else {
                    process_events(inner_events, context)?
                };
                events.extend(&evlist);
                events.push(SvgEvent::End(self.0.name.clone()));

                if self.0.name == "defs" || self.0.name == "symbol" {
                    bbox = None;
                }

                Ok((events, bbox))
            }
        } else {
            Ok((EventList::new(), None))
        }
    }
}

#[derive(Debug, Clone)]
struct OtherElement(SvgElement);

impl EventGen for OtherElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        let mut output = EventList::new();
        let mut e = self.0.clone();
        e.resolve_position(context)?; // transmute assumes some of this (e.g. dxy -> dx/dy) has been done
        e.transmute(context)?;
        e.resolve_position(context)?;
        let events = e.element_events(context)?;
        context.update_element(&e);
        if self.0.name == "point" {
            // "point" elements don't generate any events in the final output,
            // but *do* need to register themselves with update_element()
            return Ok((EventList::new(), None));
        }
        let bb = context.get_element_bbox(&e)?;
        if !events.is_empty() && bb.is_some() {
            context.set_prev_element(e.clone());
        }
        for svg_ev in events {
            let is_empty = matches!(svg_ev, SvgEvent::Empty(_));
            let adapted = if let SvgEvent::Empty(e) | SvgEvent::Start(e) = svg_ev {
                let mut bs = BytesStart::new(e.name);
                // Collect pass-through attributes
                for (k, v) in e.attrs {
                    if k != "class" && k != "data-source-line" && k != "_" && k != "__" {
                        bs.push_attribute(Attribute::from((k.as_bytes(), v.as_bytes())));
                    }
                }
                // Any 'class' attribute values are stored separately as a HashSet;
                // collect those into the BytesStart object
                if !e.classes.is_empty() {
                    bs.push_attribute(Attribute::from((
                        "class".as_bytes(),
                        e.classes
                            .into_iter()
                            .collect::<Vec<String>>()
                            .join(" ")
                            .as_bytes(),
                    )));
                }
                // Add 'data-source-line' for all elements generated by input `element`
                if context.config.add_metadata {
                    bs.push_attribute(Attribute::from((
                        "data-source-line".as_bytes(),
                        e.src_line.to_string().as_bytes(),
                    )));
                }
                let new_el = SvgElement::try_from(&bs)?;
                if is_empty {
                    SvgEvent::Empty(new_el)
                } else {
                    SvgEvent::Start(new_el)
                }
            } else {
                svg_ev
            };

            output.push(adapted);
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
    ) -> Result<(EventList, Option<BoundingBox>)> {
        // since we synthesize the opening element event here, we need to
        // do any required transformations on the <g> itself here.
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context);

        // push variables onto the stack
        context.push_element(&self.0);

        // should remove any attrs except id and classes...

        let mut result_bb = None;
        let mut events = EventList::new();
        if self.0.is_empty_element() {
            events.push(SvgEvent::Empty(new_el));
        } else {
            let el_name = new_el.name.clone();
            events.push(SvgEvent::Start(new_el));

            if let Some((start, end)) = self.0.event_range {
                let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);

                let (ev_list, bb) = process_events(inner_events, context)?;
                result_bb = bb;
                events.extend(&ev_list);
            }

            events.push(SvgEvent::End(el_name));
        }

        // pop variables off the stack
        context.pop_element();

        // Messy! should probably have a id->bbox map in context
        let mut new_el = self.0.clone();
        new_el.computed_bbox = result_bb;
        context.update_element(&new_el);
        Ok((events, result_bb))
    }
}

#[derive(Debug, Clone)]
struct ConfigElement(SvgElement);

impl EventGen for ConfigElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
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
                "font-size" => new_config.font_size = value.parse()?,
                "font-family" => new_config.font_family.clone_from(value),
                "seed" => new_config.seed = value.parse()?,
                "theme" => new_config.theme = value.parse()?,
                _ => {
                    return Err(SvgdxError::InvalidData(format!(
                        "Unknown config setting {key}"
                    )))
                }
            }
        }
        context.set_config(new_config);
        Ok((EventList::new(), None))
    }
}

#[derive(Debug, Clone)]
struct SpecsElement(SvgElement);

impl EventGen for SpecsElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        if context.in_specs {
            return Err(SvgdxError::DocumentError(
                "Nested <specs> elements are not allowed".to_string(),
            ));
        }
        if let Some((start, end)) = self.0.event_range {
            let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);
            context.in_specs = true;
            process_events(inner_events, context)?;
            context.in_specs = false;
        }
        Ok((EventList::new(), None))
    }
}

#[derive(Debug, Clone)]
struct VarElement(SvgElement);

impl EventGen for VarElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
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
                    return Err(SvgdxError::VarLimitError(format!(
                        "Variable `{}` value too long: {} (var-limit: {})",
                        key,
                        value.len(),
                        context.config.var_limit
                    )));
                }
                new_vars.push((key, value));
            }
        }
        for (k, v) in new_vars.into_iter() {
            context.set_var(&k, &v);
        }
        Ok((EventList::new(), None))
    }
}

#[derive(Debug, Clone)]
struct IfElement(SvgElement);

impl EventGen for IfElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        if let (Some(range), Some(cond)) = (self.0.event_range, self.0.get_attr("test")) {
            if eval_condition(&cond, context)? {
                // opening if element is not included in the processed inner events to avoid
                // infinite recursion...
                let (start, end) = range;
                let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);
                return process_events(inner_events.clone(), context);
            }
        }

        Ok((EventList::new(), None))
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
fn is_real_svg(events: &EventList) -> bool {
    for ev in events.iter() {
        if matches!(ev.event, Event::Start(_) | Event::Empty(_)) {
            if let Ok(el) = SvgElement::try_from(ev.clone()) {
                // "Real" SVG documents will have an `xmlns` attribute with
                // the value "http://www.w3.org/2000/svg"
                if el.name == "svg" {
                    if let Some(val) = el.get_attr("xmlns") {
                        return val == "http://www.w3.org/2000/svg";
                    }
                }
            }
            return false;
        }
    }
    false
}

#[derive(Debug, Clone)]
enum Tag {
    /// Represents a Start..End block and all events in between
    Compound(SvgElement, Option<String>),
    /// Represents a single Empty element
    Leaf(SvgElement, Option<String>),
    Comment(String, Option<String>),
    Text(String),
    CData(String),
}

impl Tag {
    fn set_text(&mut self, text: String) {
        match self {
            Tag::Compound(_, tail) => *tail = Some(text),
            Tag::Leaf(_, tail) => *tail = Some(text),
            Tag::Comment(_, tail) => *tail = Some(text),
            _ => {}
        }
    }

    fn get_element(&self) -> Option<SvgElement> {
        match self {
            Tag::Compound(el, _) => Some(el.clone()),
            Tag::Leaf(el, _) => Some(el.clone()),
            _ => None,
        }
    }
}

impl EventGen for Tag {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        let mut events = EventList::new();
        let mut bbox = None;
        match self {
            Tag::Compound(el, tail) => {
                let (ev, bb) = el.generate_events(context)?;
                (events, bbox) = (ev, bb);
                if let (Some(tail), false) = (tail, events.is_empty()) {
                    events.push(SvgEvent::Text(tail.to_owned()));
                }
            }
            Tag::Leaf(el, tail) => {
                let (ev, bb) = el.generate_events(context)?;
                (events, bbox) = (ev, bb);
                if let (Some(tail), false) = (tail, events.is_empty()) {
                    events.push(SvgEvent::Text(tail.to_owned()));
                }
            }
            Tag::Comment(c, tail) => {
                events.push(SvgEvent::Comment(c.clone()));
                if let Some(tail) = tail {
                    events.push(SvgEvent::Text(tail.to_owned()));
                }
            }
            Tag::Text(t) => {
                events.push(SvgEvent::Text(t.clone()));
            }
            Tag::CData(c) => {
                events.push(SvgEvent::CData(c.clone()));
            }
        }
        Ok((events, bbox))
    }
}

// Provide a list of tags which can be processed in-order.
fn tagify_events(events: EventList) -> Result<Vec<Tag>> {
    let mut tags = Vec::new();
    let mut ev_idx = 0;

    // we use indexed iteration as we need to skip ahead in some cases
    while ev_idx < events.len() {
        let input_ev = &events.events[ev_idx];
        ev_idx += 1;
        let ev = &input_ev.event;
        match ev {
            Event::Start(_) => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    SvgdxError::DocumentError(format!(
                        "could not extract element at line {}",
                        input_ev.line
                    ))
                })?;
                if let Some(alt_idx) = input_ev.alt_idx {
                    event_element.set_event_range((input_ev.index, alt_idx));
                    // Scan ahead to the end of this element, matching alt_idx.
                    // Note when called recursively on a subset of events, alt_idx
                    // won't be the same as next_idx, so we need to scan rather than
                    // just setting ev_idx = alt_idx + 1.
                    for next_idx in ev_idx..events.len() {
                        if events.events[next_idx].index == alt_idx {
                            ev_idx = next_idx + 1; // skip the End event itself
                            break;
                        }
                    }
                } // TODO: else warning message
                tags.push(Tag::Compound(event_element, None));
            }
            Event::Empty(_) => {
                let mut event_element = SvgElement::try_from(input_ev.clone()).map_err(|_| {
                    SvgdxError::DocumentError(format!(
                        "could not extract element at line {}",
                        input_ev.line
                    ))
                })?;
                event_element.set_event_range((input_ev.index, input_ev.index));
                tags.push(Tag::Leaf(event_element, None));
            }
            Event::Comment(c) => {
                let text = String::from_utf8(c.to_vec())?;
                tags.push(Tag::Comment(text, None));
            }
            Event::Text(t) => {
                let text = String::from_utf8(t.to_vec())?;
                if let Some(t) = tags.last_mut() {
                    t.set_text(text)
                } else {
                    tags.push(Tag::Text(text));
                }
            }
            Event::CData(c) => {
                let text = String::from_utf8(c.to_vec())?;
                if let Some(t) = tags.last_mut() {
                    t.set_text(text)
                } else {
                    tags.push(Tag::CData(text));
                }
            }
            _ => {
                // This would include Event::End, as well as PI, DocType, etc.
                // Specifically End shouldn't be seen due to alt_idx scan-ahead.
            }
        }
    }
    Ok(tags)
}

fn process_tags(
    tags: &mut Vec<(OrderIndex, Tag)>,
    context: &mut TransformerContext,
    idx_output: &mut BTreeMap<OrderIndex, EventList>,
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
    input: EventList,
    context: &mut TransformerContext,
) -> Result<(EventList, Option<BoundingBox>)> {
    if is_real_svg(&input) {
        if context.get_top_element().is_none() {
            // if this is the outermost SVG element, we mark the entire input as a 'real' SVG document
            context.real_svg = true;
        }
        return Ok((input, None));
    }
    let mut output = EventList { events: vec![] };
    let mut idx_output = BTreeMap::<OrderIndex, EventList>::new();

    let mut bbb = BoundingBoxBuilder::new();
    let mut tags = tagify_events(input)?
        .iter()
        .enumerate()
        .map(|(idx, el)| (OrderIndex::new(idx), el.clone()))
        .collect::<Vec<_>>();
    let bbox = process_tags(&mut tags, context, &mut idx_output, &mut bbb)?;

    for (_idx, events) in idx_output {
        output.events.extend(events.events);
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
        let input = EventList::from_reader(reader)?;
        self.context.set_events(input.events.clone());
        let output = process_events(input, &mut self.context)?;
        self.postprocess(output, writer)
    }

    fn write_root_svg(
        &self,
        first_svg: InputEvent,
        bbox: Option<BoundingBox>,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let mut new_svg_bs = BytesStart::new("svg");
        let mut orig_svg_attrs = HashMap::new();
        if let Event::Start(orig_svg) = first_svg.event {
            new_svg_bs = orig_svg;
            orig_svg_attrs = new_svg_bs
                .attributes()
                .map(|attr| {
                    let attr = attr.expect("Invalid SVG attribute");
                    (
                        String::from_utf8(attr.key.into_inner().to_owned()).expect("Non-UTF8"),
                        String::from_utf8(attr.value.to_vec()).expect("Non-UTF8"),
                    )
                })
                .collect();
        }
        if !orig_svg_attrs.contains_key("version") {
            new_svg_bs.push_attribute(Attribute::from(("version", "1.1")));
        }
        if !orig_svg_attrs.contains_key("xmlns") {
            new_svg_bs.push_attribute(Attribute::from(("xmlns", "http://www.w3.org/2000/svg")));
        }
        if !orig_svg_attrs.contains_key("id") {
            if let Some(local_id) = &self.context.local_style_id {
                new_svg_bs.push_attribute(Attribute::from(("id", local_id.as_str())));
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
                let new_width = format!("{}mm", width);
                let new_height = format!("{}mm", height);
                new_svg_bs.push_attribute(Attribute::from(("width", new_width.as_str())));
                new_svg_bs.push_attribute(Attribute::from(("height", new_height.as_str())));
            } else if orig_height.is_none() {
                let (width, unit) = split_unit(orig_width.expect("logic"))?;
                let new_height = format!("{}{}", fstr(width / aspect_ratio), unit);
                new_svg_bs.push_attribute(Attribute::from(("height", new_height.as_str())));
            } else if orig_width.is_none() {
                let (height, unit) = split_unit(orig_height.expect("logic"))?;
                let new_width = format!("{}{}", fstr(height * aspect_ratio), unit);
                new_svg_bs.push_attribute(Attribute::from(("width", new_width.as_str())));
            }

            if !orig_svg_attrs.contains_key("viewBox") {
                let (x1, y1) = bb.locspec(LocSpec::TopLeft);
                new_svg_bs.push_attribute(Attribute::from((
                    "viewBox",
                    format!("{} {} {} {}", fstr(x1), fstr(y1), view_width, view_height).as_str(),
                )));
            }
        }

        EventList::from(Event::Start(new_svg_bs)).write_to(writer)
    }

    fn write_auto_styles(&self, events: &mut EventList, writer: &mut dyn Write) -> Result<()> {
        // Collect the set of elements and classes so relevant styles can be
        // automatically added.
        let mut element_set = HashSet::new();
        let mut class_set = HashSet::new();
        for input_ev in events.iter() {
            let ev = &input_ev.event;
            match ev {
                Event::Start(e) | Event::Empty(e) => {
                    let ee_name = String::from_utf8(e.name().as_ref().to_vec())?;
                    element_set.insert(ee_name);
                    for attr in e.attributes() {
                        let attr = attr.map_err(SvgdxError::from_err)?;
                        let key = String::from_utf8(attr.key.as_ref().to_vec())?;
                        let value = String::from_utf8(attr.value.to_vec())?;
                        if key == "class" {
                            class_set.extend(value.split_whitespace().map(|s| s.to_owned()));
                        }
                    }
                }
                _ => {}
            }
        }

        let indent = 2;
        let mut tb = ThemeBuilder::new(&self.context, &element_set, &class_set);
        tb.build();
        let auto_defs = tb.get_defs();
        let auto_styles = tb.get_styles();
        if !auto_defs.is_empty() {
            let indent_line = format!("\n{}", " ".repeat(indent));
            let mut event_vec = vec![
                Event::Text(BytesText::new(&indent_line)),
                Event::Start(BytesStart::new("defs")),
                Event::Text(BytesText::new("\n")),
            ];
            let eee = EventList::from_str(indent_all(auto_defs, indent + 2).join("\n"))?;
            event_vec.extend(eee.events.iter().map(|e| e.event.clone()));
            event_vec.extend(vec![
                Event::Text(BytesText::new(&indent_line)),
                Event::End(BytesEnd::new("defs")),
            ]);
            let auto_defs_events = EventList::from(event_vec);
            let (before, defs_pivot, after) = events.partition("defs");
            if let Some(existing_defs) = defs_pivot {
                before.write_to(writer)?;
                auto_defs_events.write_to(writer)?;
                EventList::from(existing_defs.event).write_to(writer)?;
                *events = after;
            } else {
                auto_defs_events.write_to(writer)?;
            }
        }
        if !auto_styles.is_empty() {
            let auto_styles_events = EventList::from(vec![
                Event::Text(BytesText::new(&format!("\n{}", " ".repeat(indent)))),
                Event::Start(BytesStart::new("style")),
                Event::Text(BytesText::new(&format!("\n{}", " ".repeat(indent)))),
                Event::CData(BytesCData::new(format!(
                    "\n{}\n{}",
                    indent_all(auto_styles, indent + 2).join("\n"),
                    " ".repeat(indent)
                ))),
                Event::Text(BytesText::new(&format!("\n{}", " ".repeat(indent)))),
                Event::End(BytesEnd::new("style")),
            ]);
            let (before, style_pivot, after) = events.partition("styles");
            if let Some(existing_styles) = style_pivot {
                before.write_to(writer)?;
                auto_styles_events.write_to(writer)?;
                EventList::from(existing_styles.event).write_to(writer)?;
                *events = after;
            } else {
                auto_styles_events.write_to(writer)?;
            }
        }
        Ok(())
    }

    fn postprocess(
        &self,
        output: (EventList, Option<BoundingBox>),
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

            EventList::from(vec![
                Event::Text(BytesText::new(&indent)),
                Event::Comment(BytesText::new(&format!(
                    " Generated by {} v{} ",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                ))),
                Event::Text(BytesText::new(&indent)),
                Event::Comment(BytesText::new(&format!(
                    " Config: {:?} ",
                    self.context.config
                ))),
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
            assert!(is_real_svg(&EventList::from(input)), "{:?}", input);
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
            assert!(!is_real_svg(&EventList::from(input)), "{:?}", input);
        }
    }

    #[test]
    fn test_process_seq() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let seq = EventList::new();

        process_events(seq, &mut transformer.context).unwrap();
    }

    #[test]
    fn test_process_tags_multiple_elements() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let mut idx_output = BTreeMap::new();

        let seq = EventList::from(
            r##"<svg>
          <rect xy="#a:h" wh="10"/>
          <circle id="a" cx="50" cy="50" r="40"/>
        </svg>"##,
        );

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

        let ok_ev_count = idx_output
            .iter()
            .map(|entry| entry.1.events.len())
            .reduce(|a, b| a + b)
            .unwrap();
        assert_eq!(ok_ev_count, 7);
    }

    #[test]
    fn test_indent_all() {
        let input = vec!["a".to_string(), "  b".to_string(), "c".to_string()];
        let output = indent_all(input, 2);
        assert_eq!(output, vec!["  a", "    b", "  c"]);
    }
}
