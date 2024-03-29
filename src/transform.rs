use crate::connector::{ConnectionType, Connector};
use crate::element::ContentType;
use crate::expression::eval_attr;
use crate::svg_defs::{build_defs, build_styles};
use crate::text::process_text_attr;
use crate::types::{attr_split, attr_split_cycle, fstr, strp, BoundingBox, LocSpec, TrblLength};
use crate::{element::SvgElement, TransformConfig};

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};

use itertools::Itertools;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use anyhow::{bail, Context, Result};
use lazy_regex::regex;

use rand::prelude::*;

pub struct TransformerContext {
    elem_map: HashMap<String, SvgElement>,
    original_map: HashMap<String, SvgElement>,
    current_element: Vec<SvgElement>,
    prev_element: Option<SvgElement>,
    variables: HashMap<String, String>,
    // SmallRng is used as it is seedable.
    rng: RefCell<SmallRng>,
    real_svg: bool,
    in_specs: bool,
}

impl TransformerContext {
    pub fn new() -> Self {
        Self {
            elem_map: HashMap::new(),
            original_map: HashMap::new(),
            current_element: Vec::new(),
            prev_element: None,
            variables: HashMap::new(),
            rng: RefCell::new(SmallRng::seed_from_u64(0)),
            real_svg: false,
            in_specs: false,
        }
    }

    pub fn get_element(&self, id: &str) -> Option<&SvgElement> {
        self.elem_map.get(id)
    }

    pub fn get_original_element(&self, id: &str) -> Option<&SvgElement> {
        self.original_map.get(id)
    }

    pub fn get_element_mut(&mut self, id: &str) -> Option<&mut SvgElement> {
        self.elem_map.get_mut(id)
    }

    pub fn get_rng(&self) -> &RefCell<SmallRng> {
        &self.rng
    }

    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = RefCell::new(SmallRng::seed_from_u64(seed));
    }

    #[cfg(test)]
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.insert(name.into(), value.into());
    }

    pub fn push_current_element(&mut self, el: &SvgElement) {
        self.current_element.push(el.clone());
    }

    pub fn pop_current_element(&mut self) -> Option<SvgElement> {
        self.current_element.pop()
    }

    pub fn get_current_element(&self) -> Option<&SvgElement> {
        self.current_element.last()
    }

    pub fn get_current_element_mut(&mut self) -> Option<&mut SvgElement> {
        self.current_element.last_mut()
    }

    pub fn get_var(&self, name: &str) -> Option<String> {
        self.get_current_element()
            .and_then(|el| {
                if el.name == "var" {
                    // `var` is special - its attributes are targets, so can't
                    // also be used for lookup or `x="$x * 2"` type expressions
                    // would fail.
                    None
                } else {
                    el.get_attr(name)
                }
            })
            .or_else(|| self.variables.get(name).cloned())
    }

    pub fn get_prev_element(&self) -> Option<&SvgElement> {
        self.prev_element.as_ref()
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            if self.elem_map.insert(id.clone(), el.clone()).is_none() {
                self.original_map.insert(id, el.clone());
            }
        }
    }

    fn handle_vars(&mut self, e: &mut SvgElement) {
        // variables are updated 'in parallel' rather than one-by-one,
        // allowing e.g. swap in a single `<var>` element:
        // `<var a="$b" b="$a" />`
        let mut new_vars = HashMap::new();
        for (key, value) in e.attrs.clone() {
            // Note comments in `var` elements are permitted (and encouraged!)
            // in the input, but not propagated to the output.
            if key != "_" && key != "__" {
                let value = eval_attr(&value, self);
                new_vars.insert(key, value);
            }
        }
        self.variables.extend(new_vars);
    }

    fn handle_comments(&self, e: &mut SvgElement) -> Vec<SvgEvent> {
        let mut events = vec![];

        // Standard comment: expressions & variables are evaluated.
        if let Some(comment) = e.pop_attr("_") {
            // Expressions in comments are evaluated
            let value = eval_attr(&comment, self);
            events.push(SvgEvent::Comment(value));
            events.push(SvgEvent::Text(format!("\n{}", " ".repeat(e.indent))));
        }

        // 'Raw' comment: no evaluation of expressions occurs here
        if let Some(comment) = e.pop_attr("__") {
            events.push(SvgEvent::Comment(comment));
            events.push(SvgEvent::Text(format!("\n{}", " ".repeat(e.indent))));
        }

        events
    }

    fn handle_containment(&mut self, e: &mut SvgElement) -> Result<()> {
        let (surround, inside) = (e.pop_attr("surround"), e.pop_attr("inside"));

        if surround.is_some() && inside.is_some() {
            bail!("Cannot have 'surround' and 'inside' on an element");
        }
        if surround.is_none() && inside.is_none() {
            return Ok(());
        }

        let is_surround = surround.is_some();
        let contain_str = if is_surround { "surround" } else { "inside" };
        let ref_list = surround.unwrap_or_else(|| inside.unwrap());

        let mut bbox_list = vec![];

        for elref in attr_split(&ref_list) {
            let el = self
                .elem_map
                .get(
                    elref
                        .strip_prefix('#')
                        .context(format!("Invalid {} value {elref}", contain_str))?,
                )
                .context("Ref lookup failed at this time")?;
            {
                if let Ok(Some(el_bb)) = el.bbox() {
                    bbox_list.push(el_bb);
                } else {
                    bail!("Element #{elref} has no bounding box at this time");
                }
            }
        }
        let mut bbox = if is_surround {
            BoundingBox::union(bbox_list)
        } else {
            BoundingBox::intersection(bbox_list)
        };

        if let Some(margin) = e.pop_attr("margin") {
            let margin: TrblLength = margin.try_into()?;

            if let Some(bb) = &mut bbox {
                if is_surround {
                    bb.expand_trbl_length(margin);
                } else {
                    bb.shrink_trbl_length(margin);
                }
            }
        }
        if let Some(bb) = bbox {
            e.position_from_bbox(&bb);
        }
        e.add_class(&format!("d-{contain_str}"));
        Ok(())
    }

    /// Process a given `SvgElement` into a list of `SvgEvent`s
    ///
    /// Called once per element, and may have side-effects such
    /// as updating variable values.
    fn handle_element(&mut self, e: &SvgElement) -> Result<Vec<SvgEvent>> {
        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut events = vec![];

        let mut e = e.clone();

        if &e.name == "var" {
            self.handle_vars(&mut e);
            return Ok(vec![]);
        }
        if &e.name == "specs" {
            return Ok(vec![]);
        }
        events.extend(self.handle_comments(&mut e));
        self.handle_containment(&mut e)?;

        e.expand_attributes(self)?;

        // "xy-loc" attr allows us to position based on a non-top-left position
        // assumes the bounding-box is well-defined by this point.
        if let (Some(bbox), Some(xy_loc)) = (e.bbox()?, e.pop_attr("xy-loc")) {
            let xy_loc = LocSpec::try_from(xy_loc.as_str()).context("Invalid xy-loc value")?;
            let width = bbox.width();
            let height = bbox.height();
            let (dx, dy) = match xy_loc {
                LocSpec::TopLeft => (0., 0.),
                LocSpec::Top => (width / 2., 0.),
                LocSpec::TopRight => (width, 0.),
                LocSpec::Right => (width, height / 2.),
                LocSpec::BottomRight => (width, height),
                LocSpec::Bottom => (width / 2., height),
                LocSpec::BottomLeft => (0., height),
                LocSpec::Left => (0., height / 2.),
                LocSpec::Center => (width / 2., height / 2.),
            };
            e = e.translated(-dx, -dy)?;
            self.update_element(&e);
        }

        if e.is_connector() {
            if let Ok(conn) = Connector::from_element(
                &e,
                self,
                if let Some(e_type) = e.get_attr("edge-type") {
                    ConnectionType::from_str(&e_type)
                } else if e.name == "polyline" {
                    ConnectionType::Corner
                } else {
                    ConnectionType::Straight
                },
            ) {
                // replace with rendered connection element
                e = conn.render()?.without_attr("edge-type");
            } else {
                bail!("Cannot create connector {e}");
            }
        }

        // Process dx / dy / dxy if not a text element (where these could be useful)
        if e.name != "text" && e.name != "tspan" {
            let dx = e.pop_attr("dx");
            let dy = e.pop_attr("dy");
            let dxy = e.pop_attr("dxy");
            let mut d_x = None;
            let mut d_y = None;
            if let Some(dxy) = dxy {
                let mut parts = attr_split_cycle(&dxy).map_while(|v| strp(&v).ok());
                d_x = Some(parts.next().context("dx from dxy should be numeric")?);
                d_y = Some(parts.next().context("dy from dxy should be numeric")?);
            }
            if let Some(dx) = dx {
                d_x = Some(strp(&dx)?);
            }
            if let Some(dy) = dy {
                d_y = Some(strp(&dy)?);
            }
            if d_x.is_some() || d_y.is_some() {
                e = e.translated(d_x.unwrap_or_default(), d_y.unwrap_or_default())?;
                self.update_element(&e);
            }
        }

        if e.is_content_text() && !e.has_attr("text") {
            if let ContentType::Ready(ref value) = e.clone().content {
                e.set_attr("text", value);
            }
        }

        if e.has_attr("text") {
            let (orig_elem, text_elements) = process_text_attr(&e)?;
            prev_element = Some(e.clone());
            events.push(SvgEvent::Empty(orig_elem));
            events.push(SvgEvent::Text(format!("\n{}", " ".repeat(e.indent))));
            match text_elements.as_slice() {
                [] => {}
                [elem] => {
                    events.push(SvgEvent::Start(elem.clone()));
                    if let ContentType::Ready(value) = &elem.content {
                        events.push(SvgEvent::Text(value.clone()));
                    } else {
                        bail!("Text element should have content");
                    }
                    events.push(SvgEvent::End("text".to_string()));
                }
                _ => {
                    // Multiple text spans
                    let text_elem = &text_elements[0];
                    events.push(SvgEvent::Start(text_elem.clone()));
                    events.push(SvgEvent::Text(format!("\n{}", " ".repeat(e.indent))));
                    for elem in &text_elements[1..] {
                        // Note: we can't insert a newline/last_indent here as whitespace
                        // following a tspan is compressed to a single space and causes
                        // misalignment - see https://stackoverflow.com/q/41364908
                        events.push(SvgEvent::Start(elem.clone()));
                        if let ContentType::Ready(value) = &elem.content {
                            events.push(SvgEvent::Text(value.clone()));
                        } else {
                            bail!("Text element should have content");
                        }
                        events.push(SvgEvent::End("tspan".to_string()));
                    }
                    events.push(SvgEvent::Text(format!("\n{}", " ".repeat(e.indent))));
                    events.push(SvgEvent::End("text".to_string()));
                }
            }
            omit = true;
        }

        if !omit {
            let new_elem = e.clone();
            if new_elem.is_empty_element() {
                events.push(SvgEvent::Empty(new_elem.clone()));
            } else {
                events.push(SvgEvent::Start(new_elem.clone()));
            }
            if new_elem.bbox()?.is_some() {
                // prev_element is only used for relative positioning, so
                // only makes sense if it has a bounding box.
                prev_element = Some(new_elem);
            }
        }
        self.prev_element = prev_element;

        Ok(events)
    }
}

pub enum SvgEvent {
    Comment(String),
    Text(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputEvent<'a> {
    event: Event<'a>,
    index: usize,
    line: usize,
    indent: usize,
}

impl Clone for InputEvent<'_> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone().into_owned(),
            index: self.index,
            line: self.line,
            indent: self.indent,
        }
    }
}

impl<'a> InputEvent<'a> {
    fn into_owned(self) -> InputEvent<'static> {
        InputEvent {
            event: self.event.into_owned(),
            index: self.index,
            line: self.line,
            indent: self.indent,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventList<'a> {
    events: Vec<InputEvent<'a>>,
}

impl From<&str> for EventList<'_> {
    fn from(value: &str) -> Self {
        Self::from_str(value).expect("failed to parse string")
    }
}

impl From<Event<'_>> for EventList<'_> {
    fn from(value: Event) -> Self {
        Self {
            events: vec![InputEvent {
                event: value.into_owned(),
                index: 0,
                line: 0,
                indent: 0,
            }],
        }
    }
}

impl From<Vec<InputEvent<'_>>> for EventList<'_> {
    fn from(value: Vec<InputEvent>) -> Self {
        Self {
            events: value
                .into_iter()
                .map(|v| InputEvent {
                    event: v.event.into_owned(),
                    index: v.index,
                    line: v.line,
                    indent: v.indent,
                })
                .collect(),
        }
    }
}

impl From<Vec<Event<'_>>> for EventList<'_> {
    fn from(value: Vec<Event>) -> Self {
        Self {
            events: value
                .into_iter()
                .map(|v| InputEvent {
                    event: v.into_owned(),
                    index: 0,
                    line: 0,
                    indent: 0,
                })
                .collect(),
        }
    }
}

impl<'a> IntoIterator for EventList<'a> {
    type Item = InputEvent<'a>;
    type IntoIter = std::vec::IntoIter<InputEvent<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl From<Event<'_>> for InputEvent<'_> {
    fn from(value: Event) -> Self {
        Self {
            event: value.into_owned(),
            index: 0,
            line: 0,
            indent: 0,
        }
    }
}

impl EventList<'_> {
    fn new() -> Self {
        Self { events: vec![] }
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = &InputEvent> + '_ {
        self.events.iter()
    }

    fn len(&self) -> usize {
        self.events.len()
    }

    fn push<'a>(&mut self, ev: impl Into<InputEvent<'a>>) {
        let ev = ev.into();
        self.events.push(ev.clone().into_owned());
    }

    pub fn extend(&mut self, other: &EventList) {
        for ev in other.iter() {
            self.push(ev.event.clone());
        }
    }

    fn from_reader(reader: &mut dyn BufRead) -> Result<Self> {
        let mut reader = Reader::from_reader(reader);

        let mut events = Vec::new();
        let mut buf = Vec::new();

        let mut line_count = 1;
        let mut indent = 0;
        let mut index = 0;
        loop {
            let ev = reader.read_event_into(&mut buf);
            if let Ok(ok_ev) = ev.clone() {
                line_count += &ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count();
            }
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Text(t)) => {
                    let mut t_str = String::from_utf8(t.to_vec())?;
                    if let Some((_, rest)) = t_str.rsplit_once('\n') {
                        t_str = rest.to_string();
                    }
                    indent = t_str.len() - t_str.trim_end_matches(' ').len();

                    events.push(InputEvent {
                        event: ev.expect("match").into_owned(),
                        index,
                        line: line_count,
                        indent,
                    });
                }
                Ok(e) => events.push(InputEvent {
                    event: e.clone().into_owned(),
                    index,
                    line: line_count,
                    indent,
                }),
                Err(e) => bail!("XML error near line {}: {:?}", line_count, e),
            };

            index += 1;
            buf.clear();
        }

        Ok(Self { events })
    }

    fn from_str(s: impl Into<String>) -> Result<Self> {
        let s: String = s.into();
        let mut reader = Reader::from_str(&s);
        let mut events = Vec::new();

        // TODO: remove duplication between this and `from_reader`
        let mut line_count = 1;
        let mut indent = 0;
        let mut index = 0;
        loop {
            let ev = reader.read_event();
            if let Ok(ok_ev) = ev.clone() {
                line_count += &ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count();
            }
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Text(t)) => {
                    let t_str = String::from_utf8(t.to_vec())?;
                    if let Some((_, rest)) = t_str.rsplit_once('\n') {
                        indent = rest.len() - rest.trim_end_matches(' ').len();
                    }

                    events.push(InputEvent {
                        event: ev.expect("match").clone().into_owned(),
                        index,
                        line: line_count,
                        indent,
                    });
                }
                Ok(e) => events.push(InputEvent {
                    event: e.clone().into_owned(),
                    index,
                    line: line_count,
                    indent,
                }),
                Err(e) => bail!("XML error near line {}: {:?}", line_count, e),
            }

            index += 1;
        }
        Ok(Self { events })
    }

    fn write_to(&self, writer: &mut dyn Write) -> Result<()> {
        let mut writer = Writer::new(writer);

        let blank_line_remover = regex!("\n[ \t]+\n");
        for event_pos in &self.events {
            // trim trailing whitespace.
            // just using `trim_end()` on Text events won't work
            // as Text event may be followed by a Start/Empty event.
            // blank lines *within* Text can be trimmed.
            let mut event = event_pos.event.clone();
            if let Event::Text(t) = event {
                let mut content = String::from_utf8(t.as_ref().to_vec())?;
                content = blank_line_remover.replace_all(&content, "\n\n").to_string();
                event = Event::Text(BytesText::new(&content).into_owned());
            }
            writer.write_event(event)?;
        }
        Ok(())
    }

    /// Split an `EventList` into (up to) 3 parts: before, pivot, after.
    fn partition(&self, name: &str) -> (Self, Option<InputEvent>, Self) {
        let mut before = vec![];
        let mut pivot = None;
        let mut after = vec![];
        for input_ev in self.events.clone() {
            if pivot.is_some() {
                after.push(input_ev);
            } else {
                match input_ev.event {
                    Event::Start(ref e) | Event::Empty(ref e) => {
                        let elem_name: String =
                            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");
                        if elem_name == name {
                            pivot = Some(input_ev);
                        } else {
                            before.push(input_ev);
                        }
                    }
                    _ => before.push(input_ev),
                }
            }
        }

        (Self { events: before }, pivot, Self { events: after })
    }
}

pub struct Transformer {
    context: TransformerContext,
    config: TransformConfig,
}

impl Transformer {
    pub fn from_config(config: &TransformConfig) -> Self {
        let mut context = TransformerContext::new();
        context.seed_rng(config.seed);
        Self {
            context,
            config: config.to_owned(),
        }
    }

    pub fn transform(&mut self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let input = EventList::from_reader(reader)?;
        let output = self.process_events(input)?;
        self.postprocess(output, writer)
    }

    fn handle_svg_root(&mut self, element: &SvgElement) -> Result<()> {
        // "Real" SVG documents will have an `xmlns` attribute.
        if element.get_attr("xmlns") == Some("http://www.w3.org/2000/svg".to_owned()) {
            self.context.real_svg = true;
        }

        Ok(())
    }

    fn handle_config_element(&mut self, element: &SvgElement) -> Result<()> {
        for (key, value) in &element.attrs {
            match key.as_str() {
                "scale" => self.config.scale = value.parse()?,
                "debug" => self.config.debug = value.parse()?,
                "add-auto-styles" => self.config.add_auto_defs = value.parse()?,
                "border" => self.config.border = value.parse()?,
                "background" => self.config.background = value.clone(),
                "seed" => {
                    self.config.seed = value.parse()?;
                    self.context.seed_rng(self.config.seed);
                }
                _ => bail!("Unknown config setting {key}"),
            }
        }
        Ok(())
    }

    fn generate_element_events(&mut self, event_element: &mut SvgElement) -> Result<EventList> {
        let mut gen_events = EventList::new();
        let mut repeat = if self.context.in_specs { 0 } else { 1 };
        if let Some(rep_count) = event_element.pop_attr("repeat") {
            if event_element.is_graphics_element() {
                repeat = rep_count.parse().unwrap_or(1);
            } else {
                todo!("Repeat is not implemented for non-graphics elements");
            }
        }
        for rep_idx in 0..repeat {
            let events = transform_element(event_element, &mut self.context).context(format!(
                "processing element on line {}",
                event_element.src_line
            ));
            if let Err(err) = events {
                // TODO: save the error context with the element to show to user if it is unrecoverable.
                bail!(
                    "Error '{}' processing element on line {}",
                    err,
                    event_element.src_line
                );
            }
            let events = events?;
            if events.is_empty() {
                // if an input event doesn't generate any output events,
                // ignore text following that event to avoid blank lines in output.
                break;
            }

            for ev in events.iter() {
                gen_events.push(ev.event.clone());
            }

            if rep_idx < (repeat - 1) {
                gen_events.push(Event::Text(BytesText::new(&format!(
                    "\n{}",
                    " ".repeat(event_element.indent)
                ))));
            }
            if let Some(tail) = &event_element.tail {
                gen_events.push(Event::Text(BytesText::new(tail)));
            }
        }
        Ok(gen_events)
    }

    fn handle_reuse_element(&mut self, mut event_element: SvgElement) -> Result<SvgElement> {
        let elref = event_element
            .pop_attr("href")
            .context("reuse element should have an href attribute")?;
        let referenced_element = self
            .context
            .get_original_element(
                elref
                    .strip_prefix('#')
                    .context("href value should begin with '#'")?,
            )
            .context("unknown reference")?;
        let mut instance_element = referenced_element.clone();

        // the referenced element will have an `id` attribute (which it was
        // referenced by) but the new instance should not have this to avoid
        // multiple elements with the same id.
        // However we *do* want the instance element to inherit any `id` which
        // was on the `reuse` element.
        let ref_id = instance_element
            .pop_attr("id")
            .context("referenced element should have id")?;
        if let Some(inst_id) = event_element.pop_attr("id") {
            instance_element.set_attr("id", &inst_id);
            self.context.update_element(&event_element);
        }
        // the instanced element should have the same indent as the original
        // `reuse` element, as well as inherit `style` and `class` values.
        instance_element.set_indent(event_element.indent);
        instance_element.set_src_line(event_element.src_line);
        if let Some(inst_style) = event_element.pop_attr("style") {
            instance_element.set_attr("style", &inst_style);
        }
        instance_element.add_classes(&event_element.classes);
        instance_element.add_class(&ref_id);
        Ok(instance_element)
    }

    fn process_seq<'a>(
        &mut self,
        seq: EventList<'a>,
        idx_output: &mut BTreeMap<usize, EventList>,
    ) -> Result<EventList<'a>> {
        let mut remain = EventList::new();
        let mut last_event = None;
        let mut last_element = None;
        let mut gen_events: Vec<(usize, EventList<'_>)>;

        for input_ev in seq {
            let ev = &input_ev.event;
            gen_events = Vec::new();

            match ev {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    let is_empty = matches!(ev, Event::Empty(_));

                    let mut event_element = SvgElement::try_from(e).context(format!(
                        "could not extract element at line {}",
                        input_ev.line
                    ))?;
                    event_element.set_indent(input_ev.indent);
                    event_element.set_src_line(input_ev.line);
                    event_element.set_order_index(input_ev.index);
                    event_element.content = if is_empty {
                        ContentType::Empty
                    } else {
                        ContentType::Pending
                    };
                    last_element = Some(event_element.clone());
                    last_event = Some(ev.clone());

                    if event_element.name == "svg" && self.context.get_current_element().is_none() {
                        // The outer <svg> element is a special case.
                        self.handle_svg_root(&event_element)?;
                    }

                    if event_element.name == "config" {
                        self.handle_config_element(&event_element)?;
                        continue;
                    }

                    if event_element.name == "specs" && !is_empty {
                        if self.context.in_specs {
                            bail!("Cannot nest <specs> elements");
                        }
                        self.context.in_specs = true;
                    }

                    let mut ev_events = EventList::new();
                    self.context.update_element(&event_element);
                    if self.config.debug {
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
                    self.context.push_current_element(&event_element);
                    // support reuse element
                    if event_element.name == "reuse" {
                        match self.handle_reuse_element(event_element) {
                            Ok(ev_el) => {
                                event_element = ev_el;
                            }
                            Err(err) => {
                                self.context.pop_current_element();
                                bail!(err);
                            }
                        }
                    }
                    if is_empty {
                        let events = self.generate_element_events(&mut event_element);
                        if let Ok(ref events) = events {
                            if !events.is_empty() {
                                ev_events.extend(events);
                                gen_events.push((input_ev.index, ev_events.clone()));
                            }
                        } else {
                            remain.push(input_ev.clone());
                        }

                        self.context.pop_current_element();
                    }
                }
                Event::End(e) => {
                    let ee_name = String::from_utf8(e.name().as_ref().to_vec())?;

                    if let Some(mut event_element) = self.context.pop_current_element() {
                        if event_element.name != ee_name {
                            bail!(
                                "Mismatched end tag: expected {}, got {ee_name}",
                                event_element.name
                            );
                        }

                        if ee_name.as_str() == "specs" {
                            self.context.in_specs = false;
                        }

                        let mut events = self.generate_element_events(&mut event_element);
                        if let Ok(ref mut events) = events {
                            if !events.is_empty() {
                                // `is_content_text` elements have responsibility for handling their own text content,
                                // otherwise include the text element immediately after the opening element.
                                if !event_element.is_content_text() {
                                    if let ContentType::Ready(content) =
                                        event_element.content.clone()
                                    {
                                        events.push(Event::Text(BytesText::new(&content)));
                                    }
                                }
                                gen_events.push((event_element.order_index, events.clone()));
                                if !event_element.is_content_text() {
                                    // Similarly, `is_content_text` elements should close themselves in the returned
                                    // event list if needed.
                                    gen_events.push((input_ev.index, EventList::from(ev.clone())));
                                }
                            }
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
                            if let Some(ref mut last_element) =
                                self.context.get_current_element_mut()
                            {
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
                    if !processed && !self.context.in_specs {
                        gen_events.push((input_ev.index, EventList::from(ev.clone())));
                    }
                }
                _ => {
                    gen_events.push((input_ev.index, EventList::from(ev.clone())));
                }
            }

            for (gen_idx, gen_events) in gen_events {
                idx_output.insert(gen_idx, EventList::from(gen_events.events));
            }

            last_event = Some(ev.clone());
        }

        Ok(remain)
    }

    fn process_events<'a>(&mut self, input: EventList<'a>) -> Result<EventList<'a>> {
        let mut output = EventList { events: vec![] };
        let mut idx_output = BTreeMap::<usize, EventList>::new();

        // First pass with original input data
        let mut remain = self.process_seq(input, &mut idx_output)?;
        // Repeatedly process remaining elements while useful
        while !remain.is_empty() {
            let last_len = remain.len();
            remain = self.process_seq(remain, &mut idx_output)?;
            if last_len == remain.len() {
                bail!(
                    "Could not resolve the following elements:\n{}",
                    remain
                        .iter()
                        .map(|r| format!("{:4}: {:?}", r.line, r.event))
                        .join("\n")
                );
            }
        }

        for (_idx, events) in idx_output {
            output.events.extend(events.events);
        }

        Ok(output)
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
            extent.expand(self.config.border as f32, self.config.border as f32);
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
            if let Some(bb) = extent {
                let view_width = fstr(bb.width());
                let view_height = fstr(bb.height());
                let width = fstr(bb.width() * self.config.scale);
                let height = fstr(bb.height() * self.config.scale);
                if !orig_svg_attrs.contains(&"width".to_owned()) {
                    new_svg_bs
                        .push_attribute(Attribute::from(("width", format!("{width}mm").as_str())));
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

            EventList::from(Event::Start(new_svg_bs)).write_to(writer)?;
            output = remain;
        }

        if self.config.debug {
            let indent = "\n  ".to_owned();

            EventList::from(vec![
                Event::Text(BytesText::new(&indent)),
                Event::Comment(BytesText::new(&format!(
                    " Generated by {} v{} ",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                ))),
                Event::Text(BytesText::new(&indent)),
                Event::Comment(BytesText::new(&format!(" Config: {:?} ", self.config))),
            ])
            .write_to(writer)?;
        }

        // Default behaviour: include auto defs/styles iff we have an SVG element,
        // i.e. this is a full SVG document rather than a fragment.
        if has_svg_element && !self.context.real_svg && self.config.add_auto_defs {
            let indent = 2;
            let auto_defs = build_defs(&element_set, &class_set, &self.config);
            let auto_styles = build_styles(&element_set, &class_set, &self.config);
            if !auto_defs.is_empty() {
                let indent_line = format!("\n{}", " ".repeat(indent));
                let mut event_vec = vec![
                    Event::Text(BytesText::new(&indent_line)),
                    Event::Start(BytesStart::new("defs")),
                    Event::Text(BytesText::new("\n")),
                ];
                let eee = EventList::from_str(Self::indent_all(auto_defs, indent + 2).join("\n"))?;
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
                        Self::indent_all(auto_styles, indent + 2).join("\n"),
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
}

impl TryFrom<&BytesStart<'_>> for SvgElement {
    type Error = anyhow::Error;

    /// Build a `SvgElement` from a `BytesStart` value. Failures here are are low-level
    /// XML type errors (e.g. bad attribute names, non-UTF8) rather than anything
    /// semantic about svgdx / svg formats.
    fn try_from(e: &BytesStart) -> Result<Self, Self::Error> {
        let elem_name: String =
            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");

        let attrs: Result<Vec<(String, String)>, Self::Error> = e
            .attributes()
            .map(move |a| {
                let aa = a?;
                let key = String::from_utf8(aa.key.into_inner().to_vec())?;
                let value = aa.unescape_value()?.into_owned();
                Ok((key, value))
            })
            .collect();
        Ok(Self::new(&elem_name, &attrs?))
    }
}

/// Determine the sequence of (XML-level) events to emit in response
/// to a given `SvgElement`
fn transform_element<'a>(
    element: &'a SvgElement,
    context: &'a mut TransformerContext,
) -> Result<EventList<'a>> {
    let mut output = EventList::new();
    let ee = context.handle_element(element)?;
    for svg_ev in ee {
        // re-calculate is_empty for each generated event
        let is_empty = matches!(svg_ev, SvgEvent::Empty(_));
        match svg_ev {
            SvgEvent::Empty(e) | SvgEvent::Start(e) => {
                let mut bs = BytesStart::new(e.name);
                // Collect non-'class' attributes
                for (k, v) in e.attrs {
                    if k != "class" {
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
                if is_empty {
                    output.push(Event::Empty(bs));
                } else {
                    output.push(Event::Start(bs));
                }
            }
            SvgEvent::Comment(t) => {
                output.push(Event::Comment(BytesText::new(&t)));
            }
            SvgEvent::Text(t) => {
                output.push(Event::Text(BytesText::from_escaped(&t)));
            }
            SvgEvent::End(name) => {
                output.push(Event::End(BytesEnd::new(name)));
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eventlist_minimal() {
        let input = r#"<svg></svg>"#;
        let el = EventList::from_str(input).unwrap();
        assert_eq!(el.events.len(), 2);
        assert_eq!(el.events[0].line, 1);
        assert_eq!(el.events[0].event, Event::Start(BytesStart::new("svg")));
        assert_eq!(el.events[1].line, 1);
        assert_eq!(el.events[1].event, Event::End(BytesEnd::new("svg")));
    }

    #[test]
    fn test_eventlist_indent() {
        let input = r#"<svg>
        </svg>"#;
        let el = EventList::from_str(input).unwrap();
        assert_eq!(el.events.len(), 3);
        assert_eq!(el.events[0].line, 1);
        assert_eq!(el.events[0].indent, 0);
        assert_eq!(el.events[0].event, Event::Start(BytesStart::new("svg")));
        assert_eq!(el.events[1].line, 2);
        assert_eq!(
            el.events[1].event,
            Event::Text(BytesText::new("\n        "))
        );
        assert_eq!(el.events[2].line, 2);
        assert_eq!(el.events[2].indent, 8);
        assert_eq!(el.events[2].event, Event::End(BytesEnd::new("svg")));
    }

    #[test]
    fn test_process_seq() {
        let mut transformer = Transformer::from_config(&TransformConfig::default());
        let mut idx_output = BTreeMap::new();
        let seq = EventList::new();

        let remain = transformer.process_seq(seq, &mut idx_output);

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

        let remain = transformer.process_seq(seq, &mut idx_output);

        let ok_ev_count = idx_output
            .iter()
            .map(|entry| entry.1.events.len())
            .reduce(|a, b| a + b)
            .unwrap();
        assert_eq!(ok_ev_count, 6);
        let remain_ev_count = remain.unwrap().len();
        assert_eq!(remain_ev_count, 1);
    }
}
