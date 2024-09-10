use crate::context::{ElementMap, TransformerContext};
use crate::element::{ContentType, SvgElement};
use crate::events::{EventList, SvgEvent};
use crate::expression::{eval_attr, eval_condition};
use crate::position::{BoundingBox, LocSpec, Position};
use crate::themes::ThemeBuilder;
use crate::types::{fstr, OrderIndex};
use crate::TransformConfig;

use crate::loop_el::LoopElement;
use crate::reuse::ReuseElement;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};
use std::rc::Rc;

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
        _element: &mut SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        if let (Some(this_el), Some(parent)) = (self.get_element(), context.get_top_element()) {
            parent.borrow_mut().on_child_element(&this_el, context)?;
        }
        Ok(())
    }

    /// Determine the sequence of (XML-level) events to emit in response
    /// to a given `SvgElement`
    fn generate_events(&self, _context: &mut TransformerContext) -> Result<EventList> {
        Ok(EventList::new())
    }

    fn get_position(&self) -> Option<Position> {
        None
    }

    fn on_child_element(
        &mut self,
        _element: &SvgElement,
        _context: &mut TransformerContext,
    ) -> Result<()> {
        Ok(())
    }

    fn get_element(&self) -> Option<SvgElement> {
        None
    }

    fn get_element_mut(&mut self) -> Option<&mut SvgElement> {
        None
    }
}

impl ElementLike for SvgElement {
    fn get_element(&self) -> Option<SvgElement> {
        Some(self.clone())
    }

    fn get_element_mut(&mut self) -> Option<&mut SvgElement> {
        Some(self)
    }

    fn generate_events(&self, context: &mut TransformerContext) -> Result<EventList> {
        if self.name == "phantom" {
            return Ok(EventList::new());
        }
        let mut output = EventList::new();
        let source_line = self.get_attr("data-source-line");
        let mut e = self.clone();
        e.transmute(context)?;
        e.resolve_position(context)?;
        let events = e.element_events(context)?;
        context.update_element(&e);
        if !events.is_empty() && context.get_element_bbox(&e)?.is_some() {
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
        if context.get_top_element().is_none()
            && element.get_attr("xmlns") == Some("http://www.w3.org/2000/svg".to_owned())
        {
            context.real_svg = true;
        }

        Ok(())
    }

    fn generate_events(&self, _context: &mut TransformerContext) -> Result<EventList> {
        Ok(EventList::from(if self.0.is_empty_element() {
            SvgEvent::Empty(self.0.clone())
        } else {
            SvgEvent::Start(self.0.clone())
        }))
    }

    fn get_element(&self) -> Option<SvgElement> {
        Some(self.0.clone())
    }
}

#[derive(Debug, Clone)]
struct GroupElement {
    el: SvgElement,
    bbox: Option<BoundingBox>,
}

impl ElementLike for GroupElement {
    fn get_element(&self) -> Option<SvgElement> {
        let mut el = self.el.clone();
        el.computed_bbox = self.bbox;
        Some(el)
    }

    fn get_element_mut(&mut self) -> Option<&mut SvgElement> {
        self.el.computed_bbox = self.bbox;
        Some(&mut self.el)
    }

    fn generate_events(&self, context: &mut TransformerContext) -> Result<EventList> {
        // since we synthesize the opening element event here rather than in process_seq, we need to
        // do any required transformations on the <g> itself here.
        let mut new_el = self.el.clone();
        new_el.eval_attributes(context);
        Ok(EventList::from(if self.el.is_empty_element() {
            SvgEvent::Empty(new_el)
        } else {
            SvgEvent::Start(new_el)
        }))
    }

    fn handle_element_end(
        &mut self,
        element: &mut SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        element.computed_bbox = self.bbox;
        if let (Some(this_el), Some(parent)) = (self.get_element(), context.get_top_element()) {
            parent.borrow_mut().on_child_element(&this_el, context)?;
        }
        Ok(())
    }

    fn on_child_element(
        &mut self,
        element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        if let Ok(Some(el_bbox)) = context.get_element_bbox(element) {
            if let Some(bbox) = self.bbox {
                self.bbox = Some(bbox.combine(&el_bbox));
            } else {
                self.bbox = Some(el_bbox);
            }
        }
        Ok(())
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
                "add-auto-styles" => context.config.add_auto_styles = value.parse()?,
                "border" => context.config.border = value.parse()?,
                "background" => context.config.background.clone_from(value),
                "loop-limit" => context.config.loop_limit = value.parse()?,
                "var-limit" => context.config.var_limit = value.parse()?,
                "seed" => {
                    context.config.seed = value.parse()?;
                    context.seed_rng(context.config.seed);
                }
                "theme" => {
                    context.config.theme = value.parse()?;
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
        _element: &mut SvgElement,
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
                // Detect / prevent uncontrolled expansion of variable values
                if value.len() > context.config.var_limit as usize {
                    bail!(
                        "Variable `{}` value too long: {} (var-limit: {})",
                        key,
                        value.len(),
                        context.config.var_limit
                    );
                }
                new_vars.insert(key, value);
            }
        }
        context.variables.extend(new_vars);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct IfElement(SvgElement);

impl ElementLike for IfElement {
    fn handle_element_start(
        &mut self,
        _element: &SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        context.loop_depth += 1;
        Ok(())
    }

    fn handle_element_end(
        &mut self,
        _element: &mut SvgElement,
        context: &mut TransformerContext,
    ) -> Result<()> {
        context.loop_depth -= 1;
        Ok(())
    }

    fn get_element(&self) -> Option<SvgElement> {
        Some(self.0.clone())
    }

    fn get_element_mut(&mut self) -> Option<&mut SvgElement> {
        Some(&mut self.0)
    }

    fn generate_events(&self, context: &mut TransformerContext) -> Result<EventList> {
        if let (Some(range), Some(cond)) = (self.0.event_range, self.0.get_attr("test")) {
            if eval_condition(&cond, context)? {
                // opening if element is not included in the processed inner events to avoid
                // infinite recursion...
                let (start, end) = range;
                let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);
                return process_events(inner_events.clone(), context);
            }
        }

        Ok(EventList::new())
    }
}

impl SvgElement {
    pub fn to_ell(&self) -> Rc<RefCell<dyn ElementLike>> {
        match self.name.as_str() {
            "loop" => Rc::new(RefCell::new(LoopElement(self.clone()))), //LoopDef::try_from(element).unwrap())),
            "config" => Rc::new(RefCell::new(ConfigElement {})),
            "reuse" => Rc::new(RefCell::new(ReuseElement(self.clone()))),
            "svg" => Rc::new(RefCell::new(RootSvgElement(self.clone()))),
            "specs" => Rc::new(RefCell::new(SpecsElement {})),
            "var" => Rc::new(RefCell::new(VarElement {})),
            "if" => Rc::new(RefCell::new(IfElement(self.clone()))),
            "g" => Rc::new(RefCell::new(GroupElement {
                el: self.clone(),
                bbox: None,
            })),
            _ => Rc::new(RefCell::new(self.clone())),
        }
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
                event_element.original = String::from_utf8(e.to_owned().to_vec()).expect("utf8");
                event_element.set_indent(input_ev.indent);
                event_element.set_src_line(input_ev.line);
                event_element.set_order_index(&idx);
                event_element.content = if is_empty {
                    ContentType::Empty
                } else {
                    ContentType::Pending
                };
                // This is copied from source element to any generated elements in transform_element()
                if context.config.add_metadata && !event_element.is_phantom_element() {
                    event_element
                        .attrs
                        .insert("data-source-line".to_string(), input_ev.line.to_string());
                }
                if is_empty {
                    event_element.set_event_range((input_ev.index, input_ev.index));
                    context.update_element(&event_element);
                }
                last_element = Some(event_element.clone());

                let ell = event_element.to_ell();
                ell.borrow_mut()
                    .handle_element_start(&event_element, context)?;

                // List of events generated by *this* event.
                let mut ev_events = EventList::new();
                if is_empty {
                    let ell_ref = event_element.to_ell();
                    if context.loop_depth == 0 && !context.in_specs {
                        // TODO: for group bbox extension, we need element to have 'resolved'
                        // attributes, if possible. This is done in generate_events, but only
                        // to a local cloned object, so it doesn't get reflected here. Ideally
                        // we should split generate_events() to a 'resolve' and 'generate' phase,
                        // where only the resolve part could produce retriable reference errors.
                        let mut ok = true;
                        if let Some(el) = ell_ref.borrow_mut().get_element_mut() {
                            ok = el.resolve_position(context).is_ok();
                        }
                        let events = ell_ref.borrow_mut().generate_events(context);
                        if let Ok(ref events) = events {
                            if !events.is_empty() {
                                ev_events.extend(events);
                                gen_events.push((idx, ev_events.clone()));
                            }
                        } else {
                            ok = false;
                        }
                        if !ok {
                            remain.push(input_ev.clone());
                        }
                    }

                    ell_ref
                        .borrow_mut()
                        .handle_element_end(&mut event_element, context)?;
                } else {
                    context.push_element(ell);
                }
            }
            Event::End(e) => {
                let ee_name = String::from_utf8(e.name().as_ref().to_vec())?;

                if let Some(ell) = context.pop_element() {
                    let mut event_element = ell
                        .borrow_mut()
                        .get_element()
                        .or_else(|| Some(SvgElement::new(&ee_name, &[])))
                        .unwrap();

                    ell.borrow_mut()
                        .handle_element_end(&mut event_element, context)?;

                    let start_idx = idx_stack.pop().expect("unreachable");
                    event_element.set_event_range((start_idx, input_ev.index));
                    if let Some(eee) = ell.borrow_mut().get_element_mut() {
                        eee.set_event_range((start_idx, input_ev.index));
                    }
                    context.update_element(&event_element);

                    if event_element.name != ee_name {
                        bail!(
                            "Mismatched end tag: expected {}, got {ee_name}",
                            event_element.name
                        );
                    }

                    let mut events = if !context.in_specs && context.loop_depth == 0 {
                        ell.borrow_mut().generate_events(context)
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
                            if !(event_element.is_content_text()
                                || event_element.name == "loop"
                                || event_element.name == "if")
                            {
                                // Similarly, `is_content_text` elements should close themselves in the returned
                                // event list if needed.
                                gen_events.push((idx, EventList::from(ev.clone())));
                            }
                        }
                    } else if event_element.name == "loop" && context.loop_depth == 0 {
                        // TODO - handle 'retriable' errors separately for better error reporting
                        // currently we only handle loop separately, to ensure loop-limit works.
                        // (though potential false-positive bail on other errors inside loops...)
                        bail!("Error processing element: {events:?}");
                    } else {
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
                        if let Some(ref mut last_element) = context.get_top_element() {
                            if set_element_content_text {
                                if let Some(el) = last_element.borrow_mut().get_element_mut() {
                                    el.content = ContentType::Ready(t_str.clone());
                                }
                                processed = true;
                            }
                        }
                    }
                    Some(Event::End(_)) => {
                        // if the last *event* was an End event, the text should be
                        // set as the tail of the last *element*.
                        if let Some(ref mut last_element) = context.get_top_element() {
                            if let Some(el) = last_element.borrow_mut().get_element_mut() {
                                el.tail = Some(t_str.clone());
                            }
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
                        if let Some(bb) = self.context.get_element_bbox(&event_element)? {
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
        if has_svg_element && !self.context.real_svg && self.context.config.add_auto_styles {
            let indent = 2;
            let mut tb = ThemeBuilder::new(&self.context.config, &element_set, &class_set);
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
