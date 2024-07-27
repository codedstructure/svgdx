use crate::context::TransformerContext;
use crate::element::{ContentType, SvgElement};
use crate::events::{EventList, SvgEvent};
use crate::expression::eval_attr;
use crate::position::{BoundingBox, LocSpec};
use crate::svg_defs::{build_defs, build_styles};
use crate::types::{fstr, OrderIndex};
use crate::TransformConfig;

use crate::loop_el::LoopElement;
use crate::reuse::ReuseElement;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};

use itertools::Itertools;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};

use anyhow::{bail, Context, Result};

pub trait ElementLike: std::fmt::Debug {
    fn handle_element_start(
        &mut self,
        _element: &SvgElement,
        _context: &mut TransformerContext,
    ) -> Result<()> {
        Ok(())
    }

    fn handle_element_end(
        &mut self,
        _element: &SvgElement,
        _context: &mut TransformerContext,
    ) -> Result<()> {
        Ok(())
    }

    /// Determine the sequence of (XML-level) events to emit in response
    /// to a given `SvgElement`
    fn generate_events(&self, _context: &mut TransformerContext) -> Result<EventList> {
        Ok(EventList::new())
    }
}

impl ElementLike for SvgElement {
    fn handle_element_start(
        &mut self,
        _element: &SvgElement,
        _context: &mut TransformerContext,
    ) -> Result<()> {
        //context.push_current_element(&element);
        Ok(())
    }

    fn handle_element_end(
        &mut self,
        _element: &SvgElement,
        _context: &mut TransformerContext,
    ) -> Result<()> {
        //context.pop_current_element();
        Ok(())
    }

    fn generate_events(&self, context: &mut TransformerContext) -> Result<EventList> {
        if self.name == "phantom" {
            return Ok(EventList::new());
        }
        let mut output = EventList::new();
        let source_line = self.get_attr("data-source-line");
        let ee = context.handle_element(self)?;
        for svg_ev in ee {
            let is_empty = matches!(svg_ev, SvgEvent::Empty(_));
            let adapted = if let SvgEvent::Empty(e) | SvgEvent::Start(e) = svg_ev {
                let mut bs = BytesStart::new(e.name);
                // Collect pass-through attributes
                for (k, v) in e.attrs {
                    if k != "class" && k != "data-source-line" {
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
                if let Some(ref source_line) = source_line {
                    bs.push_attribute(Attribute::from((
                        "data-source-line".as_bytes(),
                        source_line.as_bytes(),
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
        Ok(output)
    }
}

#[derive(Debug, Clone)]
struct RootSvgElement(SvgElement);

impl ElementLike for RootSvgElement {
    fn handle_element_start(
        &mut self,
        element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        // The outer <svg> element is a special case.
        // "Real" SVG documents will have an `xmlns` attribute.
        if context.get_current_element().is_none()
            && element.get_attr("xmlns") == Some("http://www.w3.org/2000/svg".to_owned())
        {
            context.real_svg = true;
        }

        Ok(())
    }

    fn generate_events(&self, _context: &mut TransformerContext) -> Result<EventList> {
        Ok(EventList::from(SvgEvent::Start(self.0.clone())))
    }
}

#[derive(Debug, Clone)]
struct ConfigElement {}

impl ElementLike for ConfigElement {
    fn handle_element_start(
        &mut self,
        element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        for (key, value) in &element.attrs {
            match key.as_str() {
                "scale" => context.config.scale = value.parse()?,
                "debug" => context.config.debug = value.parse()?,
                "add-auto-styles" => context.config.add_auto_defs = value.parse()?,
                "border" => context.config.border = value.parse()?,
                "background" => context.config.background.clone_from(value),
                "seed" => {
                    context.config.seed = value.parse()?;
                    context.seed_rng(context.config.seed);
                }
                _ => bail!("Unknown config setting {key}"),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SpecsElement {}

impl ElementLike for SpecsElement {
    fn handle_element_start(
        &mut self,
        _element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        if context.in_specs {
            bail!("Cannot nest <specs> elements");
        }
        context.in_specs = true;
        Ok(())
    }

    fn handle_element_end(
        &mut self,
        _element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        context.in_specs = false;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct VarElement {}

impl ElementLike for VarElement {
    fn handle_element_start(
        &mut self,
        element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        if context.loop_depth > 0 {
            return Ok(());
        }
        // variables are updated 'in parallel' rather than one-by-one,
        // allowing e.g. swap in a single `<var>` element:
        // `<var a="$b" b="$a" />`
        let mut new_vars = HashMap::new();
        for (key, value) in element.attrs.clone() {
            // Note comments in `var` elements are permitted (and encouraged!)
            // in the input, but not propagated to the output.
            if key != "_" && key != "__" {
                let value = eval_attr(&value, context);
                new_vars.insert(key, value);
            }
        }
        context.variables.extend(new_vars);
        Ok(())
    }
}

fn gen_thing(element: &SvgElement) -> Box<dyn ElementLike> {
    match element.name.as_str() {
        "loop" => Box::new(LoopElement(element.clone())), //LoopDef::try_from(element).unwrap())),
        "config" => Box::new(ConfigElement {}),
        "reuse" => Box::new(ReuseElement(element.clone())),
        "svg" => Box::new(RootSvgElement(element.clone())),
        "specs" => Box::new(SpecsElement {}),
        "var" => Box::new(VarElement {}),
        _ => Box::new(element.clone()),
    }
}

fn process_seq(
    context: &mut TransformerContext,
    seq: EventList,
    idx_output: &mut BTreeMap<OrderIndex, EventList>,
) -> Result<EventList> {
    // Recursion base-case
    if seq.is_empty() {
        return Ok(EventList::new());
    }

    let mut remain = EventList::new();
    let mut last_event = None;
    let mut last_element = None;
    let mut gen_events: Vec<(OrderIndex, EventList)>;
    // Stack of event indices of open elements.
    let mut idx_stack = Vec::new();

    let init_seq_len = seq.len();

    for input_ev in seq {
        let ev = &input_ev.event;
        let idx = OrderIndex::new(input_ev.index);
        gen_events = Vec::new();

        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let is_empty = matches!(ev, Event::Empty(_));
                if !is_empty {
                    idx_stack.push(input_ev.index);
                }

                let mut event_element = SvgElement::try_from(e).context(format!(
                    "could not extract element at line {}",
                    input_ev.line
                ))?;
                event_element.set_indent(input_ev.indent);
                event_element.set_src_line(input_ev.line);
                event_element.set_order_index(&idx);
                event_element.content = if is_empty {
                    ContentType::Empty
                } else {
                    ContentType::Pending
                };
                // This is copied from source element to any generated elements in transform_element()
                if context.config.add_metadata && event_element.is_graphics_element() {
                    event_element
                        .attrs
                        .insert("data-source-line".to_string(), input_ev.line.to_string());
                }
                if is_empty {
                    event_element.set_event_range((input_ev.index, input_ev.index));
                    context.update_element(&event_element);
                }
                last_element = Some(event_element.clone());
                last_event = Some(ev.clone());

                let mut eeee = gen_thing(&event_element);
                eeee.handle_element_start(&event_element, context)?;

                if event_element.name == "config" {
                    continue;
                }

                context.push_current_element(&event_element);

                // List of events generated by *this* event.
                let mut ev_events = EventList::new();
                if context.config.debug {
                    // Prefix replaced element(s) with a representation of the original element
                    //
                    // Replace double quote with backtick to avoid messy XML entity conversion
                    // (i.e. &quot; or &apos; if single quotes were used)
                    ev_events.push(Event::Comment(BytesText::new(
                        &format!(" {event_element} ",)
                            .replace('"', "`")
                            .replace(['<', '>'], ""),
                    )));
                    ev_events.push(Event::Text(BytesText::new(&format!(
                        "\n{}",
                        " ".repeat(event_element.indent)
                    ))));
                }

                // Note this must be done before `<reuse>` processing, which 'switches out' the
                // element being processed to its target. The 'current_element' is used for
                // local variable lookup from attributes.
                //context.push_current_element(&event_element);
                // support reuse element
                let mut pop_needed = false;
                if context.loop_depth == 0 && event_element.name == "reuse" {
                    match crate::reuse::handle_reuse_element(context, event_element, idx_output) {
                        Ok(ev_el) => {
                            event_element = ev_el;
                            eeee = gen_thing(&event_element);
                            context.push_current_element(&event_element);
                            pop_needed = true;
                        }
                        Err(err) => {
                            context.pop_current_element();
                            bail!(err);
                        }
                    }
                }
                if is_empty {
                    if context.loop_depth == 0 && !context.in_specs {
                        let events = eeee.generate_events(context);
                        if let Ok(ref events) = events {
                            if !events.is_empty() {
                                ev_events.extend(events);
                                gen_events.push((idx, ev_events.clone()));
                            }
                        } else {
                            remain.push(input_ev.clone());
                        }
                    }

                    eeee.handle_element_end(&event_element, context)?;
                    context.pop_current_element();
                }
                if pop_needed {
                    // This is a bit messy, but if we pushed an extra element to support
                    // reuse, we need to pop it here. (Note we can't check for name=="reuse"
                    // here as the element has been replaced with the target element).
                    context.pop_current_element();
                }
            }
            Event::End(e) => {
                let ee_name = String::from_utf8(e.name().as_ref().to_vec())?;

                if let Some(mut event_element) = context.pop_current_element() {
                    let start_idx = idx_stack.pop().expect("unreachable");
                    event_element.set_event_range((start_idx, input_ev.index));
                    context.update_element(&event_element);

                    if event_element.name != ee_name {
                        bail!(
                            "Mismatched end tag: expected {}, got {ee_name}",
                            event_element.name
                        );
                    }

                    let mut eeee = gen_thing(&event_element);
                    eeee.handle_element_end(&event_element, context)?;

                    let mut events = if !context.in_specs {
                        eeee.generate_events(context)
                    } else {
                        Ok(EventList::new())
                    };
                    if let Ok(ref mut events) = events {
                        if !events.is_empty() {
                            // `is_content_text` elements have responsibility for handling their own text content,
                            // otherwise include the text element immediately after the opening element.
                            if !event_element.is_content_text() {
                                if let ContentType::Ready(content) = event_element.content.clone() {
                                    events.push(Event::Text(BytesText::new(&content)));
                                }
                            }
                            gen_events.push((event_element.order_index.clone(), events.clone()));
                            // TODO: this is about 'self_closing' elements include loop, g.
                            if !(event_element.is_content_text() || event_element.name == "loop") {
                                // Similarly, `is_content_text` elements should close themselves in the returned
                                // event list if needed.
                                gen_events.push((idx, EventList::from(ev.clone())));
                            }
                        }
                    } else {
                        // TODO - handle 'retriable' errors separately for better error reporting
                        remain.push(input_ev.clone());
                    }
                    last_element = Some(event_element);
                }
            }
            Event::Text(_) | Event::CData(_) => {
                // Inner value for Text and CData are different, so need to break these out again
                // into common String type.
                let t_str = match ev {
                    Event::Text(e) => String::from_utf8(e.to_vec())?,
                    Event::CData(e) => String::from_utf8(e.to_vec())?,
                    _ => panic!("unreachable"),
                };

                let mut set_element_content_text = false;
                if let Some(ref last_element) = last_element {
                    if last_element.is_phantom_element() {
                        // Ignore text following a phantom element to avoid blank lines in output.
                        continue;
                    }
                    let mut want_text = last_element.content.is_pending();
                    if matches!(ev, Event::CData(_)) {
                        // CData may happen after Text (e.g. newline+indent), in which case
                        // override any previously stored text content. (CData is used to
                        // preserve whitespace in the content text).
                        want_text |= last_element.content.is_ready();
                    }
                    set_element_content_text = last_element.is_content_text() && want_text;
                }

                let mut processed = false;
                match last_event {
                    Some(Event::Start(_)) | Some(Event::Text(_)) => {
                        // if the last *event* was a Start event, the text should be
                        // set as the content of the current *element*.
                        if let Some(ref mut last_element) = context.get_current_element_mut() {
                            if set_element_content_text {
                                last_element.content = ContentType::Ready(t_str.clone());
                                processed = true;
                            }
                        }
                    }
                    Some(Event::End(_)) => {
                        // if the last *event* was an End event, the text should be
                        // set as the tail of the last *element*.
                        if let Some(ref mut last_element) = last_element {
                            last_element.set_tail(&t_str.clone());
                        }
                    }
                    _ => {}
                }
                if !(processed || context.in_specs || context.loop_depth > 0) {
                    gen_events.push((OrderIndex::new(input_ev.index), EventList::from(ev.clone())));
                }
            }
            _ => {
                gen_events.push((OrderIndex::new(input_ev.index), EventList::from(ev.clone())));
            }
        }

        for (gen_idx, gen_events) in gen_events {
            idx_output.insert(gen_idx, EventList::from(gen_events.events));
        }

        last_event = Some(ev.clone());
    }

    if init_seq_len == remain.len() {
        bail!(
            "Could not resolve the following elements:\n{}",
            remain
                .iter()
                .map(|r| format!("{:4}: {:?}", r.line, r.event))
                .join("\n")
        );
    }

    process_seq(context, remain, idx_output)
}

pub fn process_events(input: EventList, context: &mut TransformerContext) -> Result<EventList> {
    let mut output = EventList { events: vec![] };
    let mut idx_output = BTreeMap::<OrderIndex, EventList>::new();

    process_seq(context, input, &mut idx_output)?;

    for (_idx, events) in idx_output {
        output.events.extend(events.events);
    }

    Ok(output)
}

pub struct Transformer {
    pub context: TransformerContext,
}

impl Transformer {
    pub fn from_config(config: &TransformConfig) -> Self {
        let mut context = TransformerContext::new();
        context.seed_rng(config.seed);
        context.config = config.clone();
        Self { context }
    }

    pub fn transform(&mut self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let input = EventList::from_reader(reader)?;
        self.context.set_events(input.events.clone());
        let output = process_events(input, &mut self.context)?;
        self.postprocess(output, writer)
    }

    fn postprocess(&self, mut output: EventList, writer: &mut dyn Write) -> Result<()> {
        let mut elem_path = Vec::new();
        // Collect the set of elements and classes so relevant styles can be
        // automatically added.
        let mut element_set = HashSet::new();
        let mut class_set = HashSet::new();
        // Calculate bounding box of diagram and use as new viewBox for the image.
        // This also allows just using `<svg>` as the root element.
        let mut bbox_list = vec![];
        for input_ev in output.iter() {
            let ev = &input_ev.event;
            match ev {
                Event::Start(e) | Event::Empty(e) => {
                    let ee_name = String::from_utf8(e.name().as_ref().to_vec())?;
                    element_set.insert(ee_name);
                    let is_empty = matches!(ev, Event::Empty(_));
                    let event_element = SvgElement::try_from(e)?;
                    class_set.extend(event_element.classes.to_vec());
                    if !is_empty {
                        elem_path.push(event_element.name.clone());
                    }
                    if event_element.classes.contains("background-grid") {
                        // special-case "background-grid" as an 'infinite' grid
                        // sitting behind everything...
                        continue;
                    }
                    if !(elem_path.contains(&"defs".to_string())
                        || elem_path.contains(&"symbol".to_string()))
                    {
                        if let Some(bb) = event_element.bbox()? {
                            bbox_list.push(bb);
                        }
                    }
                }
                Event::End(_) => {
                    elem_path.pop();
                }
                _ => {}
            }
        }
        // Expand by given border width
        let mut extent = BoundingBox::union(bbox_list);
        if let Some(extent) = &mut extent {
            extent.expand(
                self.context.config.border as f32,
                self.context.config.border as f32,
            );
            extent.round();
        }

        let mut has_svg_element = false;
        if let (pre_svg, Some(first_svg), remain) = output.partition("svg") {
            has_svg_element = true;
            pre_svg.write_to(writer)?;

            let mut new_svg_bs = BytesStart::new("svg");
            let mut orig_svg_attrs = vec![];
            if let Event::Start(orig_svg) = first_svg.event {
                new_svg_bs = orig_svg;
                orig_svg_attrs = new_svg_bs
                    .attributes()
                    .map(|v| {
                        String::from_utf8(v.unwrap().key.into_inner().to_owned()).expect("Non-UTF8")
                    })
                    .collect();
            }
            if !orig_svg_attrs.contains(&"version".to_owned()) {
                new_svg_bs.push_attribute(Attribute::from(("version", "1.1")));
            }
            if !orig_svg_attrs.contains(&"xmlns".to_owned()) {
                new_svg_bs.push_attribute(Attribute::from(("xmlns", "http://www.w3.org/2000/svg")));
            }
            // If width or height are provided, leave width/height/viewBox alone.
            if !orig_svg_attrs.contains(&"width".to_owned())
                && !orig_svg_attrs.contains(&"height".to_owned())
            {
                if let Some(bb) = extent {
                    let view_width = fstr(bb.width());
                    let view_height = fstr(bb.height());
                    let width = fstr(bb.width() * self.context.config.scale);
                    let height = fstr(bb.height() * self.context.config.scale);
                    if !orig_svg_attrs.contains(&"width".to_owned()) {
                        new_svg_bs.push_attribute(Attribute::from((
                            "width",
                            format!("{width}mm").as_str(),
                        )));
                    }
                    if !orig_svg_attrs.contains(&"height".to_owned()) {
                        new_svg_bs.push_attribute(Attribute::from((
                            "height",
                            format!("{height}mm").as_str(),
                        )));
                    }
                    if !orig_svg_attrs.contains(&"viewBox".to_owned()) {
                        let (x1, y1) = bb.locspec(LocSpec::TopLeft);
                        new_svg_bs.push_attribute(Attribute::from((
                            "viewBox",
                            format!("{} {} {} {}", fstr(x1), fstr(y1), view_width, view_height)
                                .as_str(),
                        )));
                    }
                }
            }

            EventList::from(Event::Start(new_svg_bs)).write_to(writer)?;
            output = remain;
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
        if has_svg_element && !self.context.real_svg && self.context.config.add_auto_defs {
            let indent = 2;
            let auto_defs = build_defs(&element_set, &class_set, &self.context.config);
            let auto_styles = build_styles(&element_set, &class_set, &self.context.config);
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
                let (before, defs_pivot, after) = output.partition("defs");
                if let Some(existing_defs) = defs_pivot {
                    before.write_to(writer)?;
                    auto_defs_events.write_to(writer)?;
                    EventList::from(existing_defs.event).write_to(writer)?;
                    output = after;
                } else {
                    auto_defs_events.write_to(writer)?;
                }
            }
            if !auto_styles.is_empty() {
                let auto_styles_events = EventList::from(vec![
                    Event::Text(BytesText::new(&format!("\n{}", " ".repeat(indent)))),
                    Event::Start(BytesStart::new("style")),
                    Event::Text(BytesText::new(&format!("\n{}", " ".repeat(indent)))),
                    Event::CData(BytesCData::new(&format!(
                        "\n{}\n{}",
                        indent_all(auto_styles, indent + 2).join("\n"),
                        " ".repeat(indent)
                    ))),
                    Event::Text(BytesText::new(&format!("\n{}", " ".repeat(indent)))),
                    Event::End(BytesEnd::new("style")),
                ]);
                let (before, style_pivot, after) = output.partition("styles");
                if let Some(existing_styles) = style_pivot {
                    before.write_to(writer)?;
                    auto_styles_events.write_to(writer)?;
                    EventList::from(existing_styles.event).write_to(writer)?;
                    output = after;
                } else {
                    auto_styles_events.write_to(writer)?;
                }
            }
        }

        output.write_to(writer)
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
    fn test_process_seq() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let mut idx_output = BTreeMap::new();
        let seq = EventList::new();

        let remain = process_seq(&mut transformer.context, seq, &mut idx_output);

        assert_eq!(remain.unwrap(), EventList::new());
    }

    #[test]
    fn test_process_seq_multiple_elements() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let mut idx_output = BTreeMap::new();

        let seq = EventList::from(
            r##"<svg>
          <rect xy="#a:h" wh="10"/>
          <circle id="a" cx="50" cy="50" r="40"/>
        </svg>"##,
        );

        let result = process_seq(&mut transformer.context, seq, &mut idx_output);
        assert!(result.is_ok());

        let ok_ev_count = idx_output
            .iter()
            .map(|entry| entry.1.events.len())
            .reduce(|a, b| a + b)
            .unwrap();
        assert_eq!(ok_ev_count, 7);
    }

    #[test]
    fn test_process_seq_slice() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let mut idx_output = BTreeMap::new();

        let seq = EventList::from(
            r##"<svg>
          <rect id="a" wh="10"/>
          <rect xy="#a:h" wh="10"/>
        </svg>"##,
        );

        let remain = process_seq(&mut transformer.context, seq.slice(2, 5), &mut idx_output);

        let ok_ev_count = idx_output
            .iter()
            .map(|entry| entry.1.events.len())
            .reduce(|a, b| a + b)
            .unwrap();
        assert_eq!(ok_ev_count, 3);
        let remain_ev_count = remain.unwrap().len();
        assert_eq!(remain_ev_count, 0);
    }

    #[test]
    fn test_indent_all() {
        let input = vec!["a".to_string(), "  b".to_string(), "c".to_string()];
        let output = indent_all(input, 2);
        assert_eq!(output, vec!["  a", "    b", "  c"]);
    }
}
