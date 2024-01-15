use crate::connector::{ConnectionType, Connector};
use crate::custom::process_custom;
use crate::expression::eval_attr;
use crate::svg_defs::{build_defs, build_styles};
use crate::text::process_text_attr;
use crate::types::{attr_split, attr_split_cycle, fstr, strp, strp_length, BoundingBox, LocSpec};
use crate::{element::SvgElement, TransformConfig};

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};

use itertools::Itertools;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use anyhow::{bail, Context, Result};
use lazy_regex::regex;

#[derive(Default)]
pub struct TransformerContext {
    elem_map: HashMap<String, SvgElement>,
    prev_element: Option<SvgElement>,
    variables: HashMap<String, String>,
    last_indent: String,
}

impl TransformerContext {
    pub fn new() -> Self {
        Self {
            elem_map: HashMap::new(),
            prev_element: None,
            variables: HashMap::new(),
            last_indent: String::new(),
        }
    }

    pub fn get_element(&self, id: &str) -> Option<&SvgElement> {
        self.elem_map.get(id)
    }

    pub fn get_element_mut(&mut self, id: &str) -> Option<&mut SvgElement> {
        self.elem_map.get_mut(id)
    }

    #[cfg(test)]
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.insert(name.into(), value.into());
    }

    pub fn get_var(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    pub fn get_prev_element(&self) -> Option<&SvgElement> {
        self.prev_element.as_ref()
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            self.elem_map.insert(id, el.clone());
        }
    }

    pub fn set_indent(&mut self, indent: String) {
        self.last_indent = indent;
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
            events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
        }

        // 'Raw' comment: no evaluation of expressions occurs here
        if let Some(comment) = e.pop_attr("__") {
            events.push(SvgEvent::Comment(comment));
            events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
        }

        events
    }

    fn handle_surround(&mut self, e: &mut SvgElement) -> Result<()> {
        if let Some(surround_list) = e.pop_attr("surround") {
            let mut bbox_list = vec![];

            for elref in attr_split(&surround_list) {
                let el = self
                    .elem_map
                    .get(
                        elref
                            .strip_prefix('#')
                            .context(format!("Invalid surround value {elref}"))?,
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
            let mut bbox = BoundingBox::combine(bbox_list);

            if let Some(margin) = e.pop_attr("margin") {
                let mut parts = attr_split_cycle(&margin).map(|v| strp_length(&v).unwrap());
                let mx = parts.next().expect("cycle");
                let my = parts.next().expect("cycle");

                if let Some(bb) = &mut bbox {
                    let bw = bb.width();
                    let h_margin = mx.adjust(bw) - bw;
                    let bh = bb.height();
                    let v_margin = my.adjust(bh) - bh;
                    bb.expand(h_margin, v_margin);
                }
            }
            if let Some(bb) = bbox {
                e.position_from_bbox(&bb);
            }
            e.add_class("d-surround");
        }

        Ok(())
    }

    /// Process a given `SvgElement` into a list of `SvgEvent`s
    ///
    /// Called once per element, and may have side-effects such
    /// as updating variable values.
    fn handle_element(&mut self, e: &SvgElement, empty: bool) -> Result<Vec<SvgEvent>> {
        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut events = vec![];

        let mut e = e.clone();

        if &e.name == "var" {
            self.handle_vars(&mut e);
            return Ok(vec![]);
        }
        events.extend(self.handle_comments(&mut e));
        self.handle_surround(&mut e)?;

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
                let mut parts = attr_split_cycle(&dxy).map(|v| strp(&v).unwrap());
                d_x = parts.next();
                d_y = parts.next();
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

        if e.has_attr("text") {
            let (orig_elem, text_elements) = process_text_attr(&e)?;
            prev_element = Some(e.clone());
            events.push(SvgEvent::Empty(orig_elem));
            events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
            match text_elements.as_slice() {
                [] => {}
                [elem] => {
                    events.push(SvgEvent::Start(elem.clone()));
                    events.push(SvgEvent::Text(
                        elem.clone().content.expect("should have content"),
                    ));
                    events.push(SvgEvent::End("text".to_string()));
                }
                _ => {
                    // Multiple text spans
                    let text_elem = &text_elements[0];
                    events.push(SvgEvent::Start(text_elem.clone()));
                    events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                    for elem in &text_elements[1..] {
                        // Note: we can't insert a newline/last_indent here as whitespace
                        // following a tspan is compressed to a single space and causes
                        // misalignment - see https://stackoverflow.com/q/41364908
                        events.push(SvgEvent::Start(elem.clone()));
                        events.push(SvgEvent::Text(
                            elem.clone().content.expect("should have content"),
                        ));
                        events.push(SvgEvent::End("tspan".to_string()));
                    }
                    events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                    events.push(SvgEvent::End("text".to_string()));
                }
            }
            omit = true;
        }

        if let Some((prev_elem, custom_events)) = process_custom(&e, empty)? {
            omit = true;
            prev_element = Some(prev_elem);
            events.extend(custom_events);
        }

        if !omit {
            let new_elem = e.clone();
            if empty {
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

#[derive(Debug, Clone)]
pub struct EventList<'a> {
    events: Vec<(Event<'a>, usize)>,
}

impl From<Event<'_>> for EventList<'_> {
    fn from(value: Event) -> Self {
        Self {
            events: vec![(value.into_owned(), 0)],
        }
    }
}

impl From<Vec<Event<'_>>> for EventList<'_> {
    fn from(value: Vec<Event>) -> Self {
        Self {
            events: value.into_iter().map(|v| (v.into_owned(), 0)).collect(),
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

    fn iter(&self) -> impl Iterator<Item = &(Event, usize)> + '_ {
        self.events.iter()
    }

    fn push(&mut self, ev: &Event) {
        self.events.push((ev.clone().into_owned(), 0));
    }

    fn from_reader(reader: &mut dyn BufRead) -> Result<Self> {
        let mut reader = Reader::from_reader(reader);

        let mut events = Vec::new();
        let mut buf = Vec::new();

        let mut line_count = 1;
        loop {
            let ev = reader.read_event_into(&mut buf);
            if let Ok(ok_ev) = ev.clone() {
                line_count += &ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count();
            }
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(e) => events.push((e.clone().into_owned(), line_count)),
                Err(e) => bail!("XML error near line {}: {:?}", line_count, e),
            };

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
        loop {
            let ev = reader.read_event();
            if let Ok(ok_ev) = ev.clone() {
                line_count += &ok_ev.as_ref().iter().filter(|&c| *c == b'\n').count();
            }
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(e) => events.push((e.clone().into_owned(), line_count)),
                Err(e) => bail!("XML error near line {}: {:?}", line_count, e),
            }
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
            let mut event = event_pos.0.clone();
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
    fn partition(&self, name: &str) -> (Self, Option<(Event, usize)>, Self) {
        let mut before = vec![];
        let mut pivot = None;
        let mut after = vec![];
        for (event, pos) in self.iter().cloned() {
            if pivot.is_some() {
                after.push((event.clone().into_owned(), pos));
            } else {
                match event {
                    Event::Start(ref e) | Event::Empty(ref e) => {
                        let elem_name: String =
                            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");
                        if elem_name == name {
                            pivot = Some((event.clone(), pos));
                        } else {
                            before.push((event.clone().into_owned(), pos));
                        }
                    }
                    _ => before.push((event.clone().into_owned(), pos)),
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
        Self {
            context: TransformerContext::new(),
            config: config.to_owned(),
        }
    }

    pub fn transform(&mut self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let input = EventList::from_reader(reader)?;
        let output = self.process_events(input)?;
        self.postprocess(output, writer)
    }

    fn process_seq<'a, 'b>(
        &mut self,
        seq: impl IntoIterator<Item = (usize, &'b Event<'b>, usize)>,
        idx_output: &mut BTreeMap<usize, EventList>,
    ) -> Result<Vec<(usize, &'b Event<'b>, usize)>> {
        let mut remain = Vec::<(usize, &Event, usize)>::new();
        let mut gen_events = EventList::new();
        let mut last_idx = 0;

        'ev: for (idx, ev, pos) in seq {
            match ev {
                Event::Start(e) | Event::Empty(e) => {
                    let is_empty = matches!(ev, Event::Empty(_));
                    let mut event_element = SvgElement::try_from(e)
                        .context(format!("could not extract element at line {pos}"))?;

                    if !gen_events.is_empty() {
                        idx_output.insert(last_idx, gen_events);
                        gen_events = EventList::new();
                    }
                    last_idx = idx;

                    self.context.update_element(&event_element);
                    if self.config.debug {
                        // Prefix replaced element(s) with a representation of the original element
                        //
                        // Replace double quote with backtick to avoid messy XML entity conversion
                        // (i.e. &quot; or &apos; if single quotes were used)
                        gen_events.push(&Event::Comment(BytesText::new(
                            &format!(" {event_element}",).replace('"', "`"),
                        )));
                        gen_events.push(&Event::Text(BytesText::new(&format!(
                            "\n{}",
                            self.context.last_indent
                        ))));
                    }
                    let mut repeat = 1;
                    if let Some(rep_count) = event_element.pop_attr("repeat") {
                        if is_empty {
                            repeat = rep_count.parse().unwrap_or(1);
                        } else {
                            todo!("Repeat is not implemented for non-empty elements");
                        }
                    }
                    for rep_idx in 0..repeat {
                        let events = transform_element(&event_element, &mut self.context, is_empty)
                            .context(format!("processing element on line {pos}"));
                        if events.is_err() {
                            remain.push((idx, ev, pos));
                            // discard any events generated by this element.
                            gen_events = EventList::new();
                            continue 'ev;
                        }
                        let events = events?;

                        for ev in events.iter() {
                            gen_events.push(&ev.0);
                        }

                        if !events.is_empty() && rep_idx < (repeat - 1) {
                            gen_events.push(&Event::Text(BytesText::new(&format!(
                                "\n{}",
                                self.context.last_indent
                            ))));
                        }
                    }
                }
                Event::End(e) => {
                    let mut ee_name = String::from_utf8(e.name().as_ref().to_vec())?;
                    if ee_name.as_str() == "tbox" {
                        ee_name = String::from("text");
                    }
                    gen_events.push(&Event::End(BytesEnd::new(ee_name)));
                    last_idx = idx;
                }
                Event::Text(e) => {
                    if gen_events.is_empty() {
                        // if a previous input event didn't generate any output events,
                        // ignore text following that event to avoid blank lines in output.
                        continue;
                    }
                    // Extract any trailing whitespace following newlines as the current indentation level
                    let re = regex!(r"(?ms)\n.*^(\s+)*");
                    let text = String::from_utf8(e.to_vec()).expect("Non-UTF8 in input file");
                    if let Some(captures) = re.captures(&text) {
                        let indent = captures.get(1).map_or(String::new(), |m| m.as_str().into());
                        self.context.set_indent(indent);
                    }

                    gen_events.push(&Event::Text(e.borrow()));
                }
                _ => {
                    gen_events.push(ev);
                }
            }
        }

        if !gen_events.is_empty() {
            idx_output.insert(last_idx, gen_events);
        }
        Ok(remain)
    }

    fn process_events<'a>(&mut self, input: EventList<'a>) -> Result<EventList<'a>> {
        let mut output = EventList { events: vec![] };
        let mut idx_output = BTreeMap::<usize, EventList>::new();

        let seq = input.iter().enumerate().map(|x| (x.0, &x.1 .0, x.1 .1));

        // First pass with original input data
        let mut remain = self.process_seq(seq, &mut idx_output)?;
        // Repeatedly process remaining elements while useful
        while !remain.is_empty() {
            let last_len = remain.len();
            remain = self.process_seq(remain, &mut idx_output)?;
            if last_len == remain.len() {
                bail!(
                    "Could not resolve the following elements:\n{}",
                    remain
                        .iter()
                        .map(|r| format!("{:4}: {:?}", r.2, r.1))
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
        for (ev, _) in output.iter() {
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
        let mut extent = BoundingBox::combine(bbox_list);
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
            if let (Event::Start(orig_svg), _) = first_svg {
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
            let mut indent = self.context.last_indent.clone();
            if indent.is_empty() {
                indent = "\n  ".to_owned();
            }

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
        if has_svg_element && self.config.add_auto_defs {
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
                event_vec.extend(eee.events.iter().map(|e| e.0.clone()));
                event_vec.extend(vec![
                    Event::Text(BytesText::new(&indent_line)),
                    Event::End(BytesEnd::new("defs")),
                ]);
                let auto_defs_events = EventList::from(event_vec);
                let (before, defs_pivot, after) = output.partition("defs");
                if let Some((existing_defs, _)) = defs_pivot {
                    before.write_to(writer)?;
                    auto_defs_events.write_to(writer)?;
                    EventList::from(existing_defs).write_to(writer)?;
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
                if let Some((existing_styles, _)) = style_pivot {
                    before.write_to(writer)?;
                    auto_styles_events.write_to(writer)?;
                    EventList::from(existing_styles).write_to(writer)?;
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
    is_empty: bool,
) -> Result<EventList<'a>> {
    let mut output = EventList::new();
    let ee = context.handle_element(element, is_empty)?;
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
                    output.push(&Event::Empty(bs));
                } else {
                    output.push(&Event::Start(bs));
                }
            }
            SvgEvent::Comment(t) => {
                output.push(&Event::Comment(BytesText::new(&t)));
            }
            SvgEvent::Text(t) => {
                output.push(&Event::Text(BytesText::new(&t)));
            }
            SvgEvent::End(name) => {
                output.push(&Event::End(BytesEnd::new(name)));
            }
        }
    }
    Ok(output)
}
