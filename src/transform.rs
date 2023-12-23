use crate::connector::{ConnectionType, Connector};
use crate::custom::process_custom;
use crate::expression::eval_attr;
use crate::svg_defs;
use crate::text::process_text_attr;
use crate::types::BoundingBox;
use crate::{attr_split_cycle, element::SvgElement, fstr, strp, strp_length};

use std::collections::HashMap;
use std::io::{BufRead, Write};

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use anyhow::{bail, Context, Result};
use regex::Regex;

#[derive(Clone, Default, Debug)]
pub struct TransformerContext {
    pub(crate) elem_map: HashMap<String, SvgElement>,
    pub(crate) prev_element: Option<SvgElement>,
    pub(crate) variables: HashMap<String, String>,
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

    fn populate(&mut self, events: &EventList) -> Result<()> {
        let mut elem_map: HashMap<String, SvgElement> = HashMap::new();

        for (ev, pos) in events.iter() {
            match ev {
                Event::Eof => {
                    // should never happen, as handled in EventList::from_reader()
                    break;
                }
                Event::Start(e) | Event::Empty(e) => {
                    let elem_name: String =
                        String::from_utf8(e.name().into_inner().to_vec()).unwrap();
                    let mut attr_list = vec![];
                    let mut id_opt = None;
                    for a in e.attributes() {
                        let aa = a.context(format!("Invalid attribute at {pos}"))?;

                        let key =
                            String::from_utf8(aa.key.into_inner().to_vec()).expect("not UTF8");
                        let value = aa.unescape_value().expect("XML decode error").into_owned();

                        if &key == "id" {
                            id_opt = Some(value);
                        } else {
                            attr_list.push((key, value.clone()));
                        }
                    }
                    if let Some(id) = id_opt {
                        let mut elem = SvgElement::new(&elem_name, &attr_list);
                        // Expand anything we can given the current context
                        elem.expand_attributes(true, self)?;
                        elem_map.insert(id.clone(), elem);
                    }
                }
                _ => {}
            }
        }
        self.elem_map = elem_map;

        Ok(())
    }

    pub(crate) fn set_indent(&mut self, indent: String) {
        self.last_indent = indent;
    }

    fn update_elem_map(elem_map: &mut HashMap<String, SvgElement>, e: &SvgElement) {
        if let Some(el_id) = e.get_attr("id") {
            elem_map.insert(el_id, e.clone());
        }
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
            // variables are updated 'in parallel' rather than one-by-one,
            // allowing e.g. swap in a single `<var>` element:
            // `<var a="$b" b="$a" />`
            let mut new_vars = HashMap::new();
            for (key, value) in e.attrs.clone() {
                let value = eval_attr(&value, &self.variables, &self.elem_map);
                new_vars.insert(key, value);
            }
            self.variables.extend(new_vars);
            return Ok(vec![]);
        }

        e.expand_attributes(false, self)?;

        // Size adjustments must be computed before updating position,
        // as they affect any xy-loc other than default top-left.
        // NOTE: these attributes may be removed once variable arithmetic
        // is implemented; currently key use-case is e.g. wh="$var" dw="-4"
        // with $var="20 30" or similar (the reference form of wh already
        // supports inline dw / dh).
        {
            let dw = e.pop_attr("dw");
            let dh = e.pop_attr("dh");
            let dwh = e.pop_attr("dwh");
            let mut d_w = None;
            let mut d_h = None;
            if let Some(dwh) = dwh {
                let mut parts = attr_split_cycle(&dwh).map(|v| strp_length(&v).unwrap());
                d_w = parts.next();
                d_h = parts.next();
            }
            if let Some(dw) = dw {
                d_w = Some(strp_length(&dw)?);
            }
            if let Some(dh) = dh {
                d_h = Some(strp_length(&dh)?);
            }
            if d_w.is_some() || d_h.is_some() {
                e = e.resized_by(d_w.unwrap_or_default(), d_h.unwrap_or_default());
                Self::update_elem_map(&mut self.elem_map, &e);
            }
        }

        // "xy-loc" attr allows us to position based on a non-top-left position
        // assumes the bounding-box is well-defined by this point.
        if let (Some(bbox), Some(xy_loc)) = (e.bbox()?, e.pop_attr("xy-loc")) {
            let width = bbox.width().unwrap();
            let height = bbox.height().unwrap();
            let (dx, dy) = match xy_loc.as_str() {
                "tl" => (0., 0.),
                "t" => (width / 2., 0.),
                "tr" => (width, 0.),
                "r" => (width, height / 2.),
                "br" => (width, height),
                "b" => (width / 2., height),
                "bl" => (0., height),
                "l" => (0., height / 2.),
                "c" => (width / 2., height / 2.),
                _ => (0., 0.),
            };
            e = e.translated(-dx, -dy);
            Self::update_elem_map(&mut self.elem_map, &e);
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
            }
        }

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
                e = e.translated(d_x.unwrap_or_default(), d_y.unwrap_or_default());
                Self::update_elem_map(&mut self.elem_map, &e);
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
                    events.push(SvgEvent::Text(elem.clone().content.unwrap()));
                    events.push(SvgEvent::End("text".to_string()));
                }
                _ => {
                    let text_elem = &text_elements[0];
                    events.push(SvgEvent::Start(text_elem.clone()));
                    events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                    for elem in &text_elements[1..] {
                        // Note: we can't insert a newline/last_indent here as whitespace
                        // following a tspan is compressed to a single space and causes
                        // misalignment - see https://stackoverflow.com/q/41364908
                        events.push(SvgEvent::Start(elem.clone()));
                        events.push(SvgEvent::Text(elem.clone().content.unwrap()));
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
            e.expand_attributes(true, self)?;
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

#[derive(Debug)]
pub enum SvgEvent {
    Text(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
}

#[derive(Debug, Clone)]
struct EventList<'a> {
    events: Vec<(Event<'a>, usize)>,
}

impl<'a> EventList<'a> {
    fn new() -> Self {
        Self { events: vec![] }
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn len(&self) -> usize {
        self.events.len()
    }
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

        for event in &self.events {
            writer.write_event(event.0.clone())?;
        }
        Ok(())
    }

    /// Split an EventList into (up to) 3 parts: before, pivot, after.
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

    fn iter(&self) -> impl Iterator<Item = &(Event, usize)> + '_ {
        self.events.iter()
    }

    fn push(&mut self, ev: &Event) {
        self.events.push((ev.clone().into_owned(), 0));
    }
}

#[derive(Default, Debug)]
pub struct Transformer {
    context: TransformerContext,
    debug: bool,
    add_defs: bool,
    add_style: bool,
}

impl Transformer {
    pub fn new() -> Self {
        Self {
            context: TransformerContext::new(),
            // TODO: expose these
            debug: false,
            add_defs: false,
            add_style: false,
        }
    }

    pub fn transform(&mut self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let input = EventList::from_reader(reader)?;
        let mut output = EventList { events: vec![] };

        self.context.populate(&input)?;

        let mut changed_output = false;
        for (ev, pos) in input.iter() {
            let old_len = output.len();
            match ev {
                Event::Eof => {
                    // should never happen, as handled in EventList::from_reader()
                    break;
                }
                Event::Start(e) | Event::Empty(e) => {
                    let is_empty = matches!(ev, Event::Empty(_));
                    let mut repeat = 1;
                    let mut event_element = SvgElement::try_from(e)
                        .context(format!("could not extract element at line {pos}"))?;
                    if let Some(rep_count) = event_element.pop_attr("repeat") {
                        if is_empty {
                            repeat = rep_count.parse().unwrap_or(1);
                        } else {
                            todo!("Repeat is not implemented for non-empty elements");
                        }
                    }
                    if self.debug {
                        // Prefix replaced element(s) with a representation of the original element
                        //
                        // Replace double quote with backtick to avoid messy XML entity conversion
                        // (i.e. &quot; or &apos; if single quotes were used)
                        output.push(&Event::Comment(BytesText::new(
                            &format!(" {event_element}",).replace('"', "`"),
                        )));
                        output.push(&Event::Text(BytesText::new(&format!(
                            "\n{}",
                            self.context.last_indent
                        ))));
                    }
                    for rep_idx in 0..repeat {
                        let events = transform_element(&event_element, &mut self.context, is_empty)
                            .context(format!("processing element on line {pos}"))?;
                        for ev in events.iter() {
                            output.push(&ev.0);
                        }

                        if !events.is_empty() && rep_idx < (repeat - 1) {
                            output.push(&Event::Text(BytesText::new(&format!(
                                "\n{}",
                                self.context.last_indent
                            ))));
                        }
                    }
                }
                Event::End(e) => {
                    let mut ee_name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                    if ee_name.as_str() == "tbox" {
                        ee_name = String::from("text");
                    }
                    output.push(&Event::End(BytesEnd::new(ee_name)));
                }
                Event::Text(e) => {
                    if !changed_output {
                        // if a previous input event didn't generate any
                        // output events, ignore any text following that
                        // input event.
                        continue;
                    }
                    // Extract any trailing whitespace following newlines as the current indentation level
                    let re = Regex::new(r"(?ms)\n.*^(\s+)*").expect("Bad Regex");
                    let text = String::from_utf8(e.to_vec()).expect("Non-UTF8 in input file");
                    if let Some(captures) = re.captures(&text) {
                        let indent = captures.get(1).map_or(String::new(), |m| m.as_str().into());
                        self.context.set_indent(indent);
                    }

                    output.push(&Event::Text(e.borrow()));
                }
                _ => {
                    output.push(ev);
                }
            }
            changed_output = output.len() != old_len;
        }

        // Calculate bounding box of diagram and use as new viewBox for the image.
        // This also allows just using `<svg>` as the root element.
        let mut extent = BoundingBox::new();
        let mut elem_path = Vec::new();
        for (ev, _) in output.iter() {
            match ev {
                Event::Start(e) | Event::Empty(e) => {
                    let is_empty = matches!(ev, Event::Empty(_));
                    let event_element = SvgElement::try_from(e)?;
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
                        if let Some(bb) = &event_element.bbox()? {
                            extent.extend(bb);
                        }
                    }
                }
                Event::End(_) => {
                    elem_path.pop();
                }
                _ => {}
            }
        }
        // Expand by 5, then add 5%. Ensures small images get more than a couple
        // of pixels border, and large images still get a (relatively) decent border.
        extent.expand(5.);
        extent.scale(1.05);
        extent.round();

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
            if let BoundingBox::BBox(minx, miny, maxx, maxy) = extent {
                let width = fstr(maxx - minx);
                let height = fstr(maxy - miny);
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
                    new_svg_bs.push_attribute(Attribute::from((
                        "viewBox",
                        format!("{} {} {} {}", fstr(minx), fstr(miny), width, height).as_str(),
                    )));
                }
            }

            EventList::from(Event::Start(new_svg_bs)).write_to(writer)?;
            output = remain;
        }

        // Default behaviour: include default styles iff we have an SVG element,
        // i.e. this is a full SVG document rather than a fragment.
        if has_svg_element {
            if self.add_defs {
                output = Self::include_defaults(
                    "defs",
                    &EventList::from_str(svg_defs::SVG_DEFS).expect("Bad internal SVG definition"),
                    &output,
                    writer,
                )?
                .clone();
            }
            if self.add_style {
                output = Self::include_defaults(
                    "style",
                    &EventList::from_str(svg_defs::SVG_STYLES)
                        .expect("Bad internal SVG definition"),
                    &output,
                    writer,
                )?
                .clone();
            }
        }

        output.write_to(writer)
    }

    fn include_defaults<'a>(
        name: &str,
        def_content: &EventList<'a>,
        ev_list: &EventList<'a>,
        writer: &mut dyn Write,
    ) -> Result<EventList<'a>> {
        let (before, style, after) = ev_list.partition(name);
        if let Some((def_start_event, _)) = style {
            before.write_to(writer)?;
            def_content.write_to(writer)?;
            EventList::from(def_start_event).write_to(writer)?;
            Ok(after.clone())
        } else {
            // No existing style element, so we'll write one out immediately
            // (having just written out the svg element).
            def_content.write_to(writer)?;
            Ok(before.clone())
        }
    }
}

impl TryFrom<&BytesStart<'_>> for SvgElement {
    type Error = anyhow::Error;

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
        Ok(SvgElement::new(&elem_name, &attrs?))
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
