use crate::connector::{ConnectionType, Connector};
use crate::types::BoundingBox;
use crate::{fstr, SvgElement};

use std::collections::HashMap;
use std::io::{BufReader, Read, Write};

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use regex::Regex;

#[derive(Clone, Default, Debug)]
pub(crate) struct TransformerContext {
    pub(crate) elem_map: HashMap<String, SvgElement>,
    pub(crate) prev_element: Option<SvgElement>,
    last_indent: String,
}

impl TransformerContext {
    pub fn new() -> Self {
        let elem_map: HashMap<String, SvgElement> = HashMap::new();

        Self {
            elem_map,
            prev_element: None,
            last_indent: String::from(""),
        }
    }

    fn populate(&mut self, events: &EventList) {
        let mut elem_map: HashMap<String, SvgElement> = HashMap::new();

        for ev in events.iter() {
            match ev {
                Event::Eof => {
                    // should never happen, as handled in EventList::from_reader()
                    break;
                }
                Event::Start(e) | Event::Empty(e) => {
                    let mut attr_list = vec![];
                    let mut id_opt = None;
                    for a in e.attributes() {
                        let aa = a.unwrap();

                        let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                        let value = aa.unescape_value().unwrap().into_owned();

                        if &key == "id" {
                            id_opt = Some(value);
                        } else {
                            attr_list.push((key, value.clone()));
                        }
                    }
                    if let Some(id) = id_opt {
                        let elem_name: String =
                            String::from_utf8(e.name().into_inner().to_vec()).unwrap();
                        let mut elem = SvgElement::new(&elem_name, &attr_list);
                        // Expand anything we can given the current context
                        elem.expand_attributes(true, self);
                        elem_map.insert(id.clone(), elem);
                    }
                }
                _ => {}
            }
        }
        self.elem_map = elem_map;
    }

    pub(crate) fn set_indent(&mut self, indent: String) {
        self.last_indent = indent;
    }

    fn eval_ref(&self, attr: &str) -> Option<(f32, f32)> {
        // Example: "#thing@tl" => top left coordinate of element id="thing"
        let re = Regex::new(r"^#(?<id>[^@]+)(@(?<loc>\S+))?$").unwrap();

        let input = String::from(attr);

        let caps = re.captures(&input)?;
        let name = &caps["id"];
        let loc = caps.name("loc").map_or("", |v| v.as_str());
        if loc.is_empty() {
            // find nearest location to us
        }

        let element = self.elem_map.get(name)?;
        element.coord(loc)
    }

    pub(crate) fn closest_loc(&self, this: &SvgElement, point: (f32, f32)) -> String {
        let mut min_dist_sq = f32::MAX;
        let mut min_loc = "c";

        for loc in this.locations() {
            let this_coord = this.coord(loc);
            if let (Some((x1, y1)), (x2, y2)) = (this_coord, point) {
                let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
                if dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                    min_loc = loc;
                }
            }
        }
        min_loc.to_string()
    }

    pub(crate) fn shortest_link(&self, this: &SvgElement, that: &SvgElement) -> (String, String) {
        let mut min_dist_sq = f32::MAX;
        let mut this_min_loc = "c";
        let mut that_min_loc = "c";
        for this_loc in this.locations() {
            for that_loc in that.locations() {
                let this_coord = this.coord(this_loc);
                let that_coord = that.coord(that_loc);
                if let (Some((x1, y1)), Some((x2, y2))) = (this_coord, that_coord) {
                    let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
                    if dist_sq < min_dist_sq {
                        min_dist_sq = dist_sq;
                        this_min_loc = this_loc;
                        that_min_loc = that_loc;
                    }
                }
            }
        }
        (this_min_loc.to_owned(), that_min_loc.to_owned())
    }

    fn handle_element(&mut self, e: &SvgElement, empty: bool) -> Vec<SvgEvent> {
        let elem_name = &e.name;

        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut events = vec![];

        let mut e = e.clone();

        e.expand_attributes(false, self);

        if e.is_connector() {
            let conn = Connector::from_element(
                &e,
                self,
                if e.name == "polyline" {
                    ConnectionType::Corner
                } else {
                    ConnectionType::Straight
                },
            );
            // replace with rendered connection element
            e = conn.render();
        }

        if let Some((orig_elem, text_elements)) = e.process_text_attr() {
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

        // Expand any custom element types
        match elem_name.as_str() {
            "tbox" => {
                let mut rect_attrs = vec![];
                let mut text_attrs = vec![];

                let mut text = None;

                for (key, value) in e.clone().attrs {
                    if key == "text" {
                        // allows an empty element to contain text content directly as an attribute
                        text = Some(value);
                    } else {
                        rect_attrs.push((key.clone(), value.clone()));
                    }
                }

                let rect_elem = SvgElement::new("rect", &rect_attrs).add_class("tbox");
                // Assumption is that text should be centered within the rect,
                // and has styling via CSS to reflect this, e.g.:
                //  text.tbox { dominant-baseline: central; text-anchor: middle; }
                let cxy = rect_elem.bbox().unwrap().center().unwrap();
                text_attrs.push(("x".into(), fstr(cxy.0)));
                text_attrs.push(("y".into(), fstr(cxy.1)));
                prev_element = Some(rect_elem.clone());
                events.push(SvgEvent::Empty(rect_elem));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                let text_elem = SvgElement::new("text", &text_attrs).add_class("tbox");
                events.push(SvgEvent::Start(text_elem));
                // if this *isn't* empty, we'll now expect a text event, which will be passed through.
                // the corresponding </tbox> will be converted into a </text> element.
                if empty {
                    if let Some(tt) = text {
                        events.push(SvgEvent::Text(tt.to_string()));
                        events.push(SvgEvent::End("text".to_string()));
                    }
                }
                omit = true;
            }
            "person" => {
                let mut h: f32 = 100.;
                let mut x1: f32 = 0.;
                let mut y1: f32 = 0.;
                let mut common_attrs = vec![];
                let mut head_attrs = vec![];
                let mut body_attrs = vec![];
                let mut arms_attrs = vec![];
                let mut leg1_attrs: Vec<(String, String)> = vec![];
                let mut leg2_attrs: Vec<(String, String)> = vec![];

                for (key, value) in e.clone().attrs {
                    match key.as_str() {
                        "x" => {
                            x1 = value.clone().parse().unwrap();
                        }
                        "y" => {
                            y1 = value.clone().parse().unwrap();
                        }
                        "height" => {
                            h = value.clone().parse().unwrap();
                        }
                        _ => {
                            common_attrs.push((key.clone(), value.clone()));
                        }
                    }
                }

                head_attrs.push(("cx".into(), fstr(x1 + h / 6.)));
                head_attrs.push(("cy".into(), fstr(y1 + h / 9.)));
                head_attrs.push(("r".into(), fstr(h / 9.)));
                head_attrs.push(("style".into(), "fill:none; stroke-width:0.5".into()));
                head_attrs.extend(common_attrs.clone());

                body_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
                body_attrs.push(("y1".into(), fstr(y1 + h / 9. * 2.)));
                body_attrs.push(("x2".into(), fstr(x1 + h / 6.)));
                body_attrs.push(("y2".into(), fstr(y1 + (5.5 * h) / 9.)));
                body_attrs.extend(common_attrs.clone());

                arms_attrs.push(("x1".into(), fstr(x1)));
                arms_attrs.push(("y1".into(), fstr(y1 + h / 3.)));
                arms_attrs.push(("x2".into(), fstr(x1 + h / 3.)));
                arms_attrs.push(("y2".into(), fstr(y1 + h / 3.)));
                arms_attrs.extend(common_attrs.clone());

                leg1_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
                leg1_attrs.push(("y1".into(), fstr(y1 + (5.5 * h) / 9.)));
                leg1_attrs.push(("x2".into(), fstr(x1)));
                leg1_attrs.push(("y2".into(), fstr(y1 + h)));
                leg1_attrs.extend(common_attrs.clone());

                leg2_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
                leg2_attrs.push(("y1".into(), fstr(y1 + (5.5 * h) / 9.)));
                leg2_attrs.push(("x2".into(), fstr(x1 + h / 3.)));
                leg2_attrs.push(("y2".into(), fstr(y1 + h)));
                leg2_attrs.extend(common_attrs.clone());

                events.push(SvgEvent::Empty(
                    SvgElement::new("circle", &head_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &body_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &arms_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &leg1_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &leg2_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));

                omit = true;
            }
            "pipeline" => {
                let mut x = 0.;
                let mut y = 0.;
                let mut width = 0.;
                let mut height = 0.;
                let mut common_attrs = vec![];
                for (key, value) in e.clone().attrs {
                    match key.as_str() {
                        "x" => {
                            x = value.clone().parse().unwrap();
                        }
                        "y" => {
                            y = value.clone().parse().unwrap();
                        }
                        "height" => {
                            height = value.clone().parse().unwrap();
                        }
                        "width" => {
                            width = value.clone().parse().unwrap();
                        }
                        _ => {
                            common_attrs.push((key.clone(), value.clone()));
                        }
                    }
                }

                if width < height {
                    // Vertical pipeline
                    let w_by2 = width / 2.;
                    let w_by4 = width / 4.;

                    common_attrs.push((
                        "d".to_string(),
                        format!(
                    "M {} {} a {},{} 0 0,0 {},0 a {},{} 0 0,0 -{},0 v {} a {},{} 0 0,0 {},0 v -{}",
                    x, y + w_by4,
                    w_by2, w_by4, width,
                    w_by2, w_by4, width,
                    height - w_by2,
                    w_by2, w_by4, width,
                    height - w_by2),
                    ));
                } else {
                    // Horizontal pipeline
                    let h_by2 = height / 2.;
                    let h_by4 = height / 4.;

                    common_attrs.push((
                        "d".to_string(),
                        format!(
                    "M {} {} a {},{} 0 0,0 0,{} a {},{} 0 0,0 0,-{} h {} a {},{} 0 0,1 0,{} h -{}",
                    x + h_by4, y,
                    h_by4, h_by2, height,
                    h_by4, h_by2, height,
                    width - h_by2,
                    h_by4, h_by2, height,
                    width - h_by2),
                    ));
                }
                events.push(SvgEvent::Empty(
                    SvgElement::new("path", &common_attrs).add_class("pipeline"),
                ));

                omit = true;
                // Since we're omitting the original element we need to set a separate
                // element to act as the previous element for relative positioning
                let bbox = SvgElement::new(
                    "rect",
                    &[
                        ("x".into(), fstr(x)),
                        ("y".into(), fstr(y)),
                        ("width".into(), fstr(width)),
                        ("height".into(), fstr(height)),
                    ],
                );
                prev_element = Some(bbox);
            }
            _ => {}
        }

        if !omit {
            e.expand_attributes(true, self);
            let new_elem = e.clone();
            if empty {
                events.push(SvgEvent::Empty(new_elem.clone()));
            } else {
                events.push(SvgEvent::Start(new_elem.clone()));
            }
            if new_elem.bbox().is_some() {
                // prev_element is only used for relative positioning, so
                // only makes sense if it has a bounding box.
                prev_element = Some(new_elem.clone());
            }
        }
        self.prev_element = prev_element;

        events
    }
}

#[derive(Debug)]
enum SvgEvent {
    Text(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
}

struct EventList<'a> {
    events: Vec<Event<'a>>,
}

impl EventList<'_> {
    fn from_reader(reader: &mut dyn Read) -> Self {
        let mut reader = Reader::from_reader(BufReader::new(reader));

        let mut events = Vec::new();
        let mut buf = Vec::new();

        loop {
            let ev = reader.read_event_into(&mut buf);
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(e) => events.push(e.clone().into_owned()),
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            };

            buf.clear();
        }

        Self { events }
    }

    fn write_to(&self, writer: &mut dyn Write) -> Result<(), String> {
        let mut writer = Writer::new(writer);

        for event in &self.events {
            writer.write_event(event).map_err(|e| e.to_string())?
        }
        Ok(())
    }

    fn iter(&self) -> impl Iterator<Item = &Event> + '_ {
        self.events.iter()
    }

    fn to_elements(&self) -> Vec<SvgElement> {
        let mut result = vec![];
        let mut event_iter = self.iter();
        loop {
            match event_iter.next() {
                Some(Event::Empty(e)) | Some(Event::Start(e)) => {
                    result.push(e.into());
                }
                None => {
                    break;
                }
                _ => (),
            }
        }
        result
    }

    fn push(&mut self, ev: &Event) {
        self.events.push(ev.clone().into_owned());
    }
}

#[derive(Default, Debug)]
pub struct Transformer {
    context: TransformerContext,
}

impl Transformer {
    pub fn new() -> Self {
        Self {
            context: TransformerContext::new(),
        }
    }

    pub fn transform(
        &mut self,
        reader: &mut dyn Read,
        writer: &mut dyn Write,
    ) -> Result<(), String> {
        let input = EventList::from_reader(reader);
        let mut output = EventList { events: vec![] };

        self.context.populate(&input);

        for ev in input.iter() {
            match ev {
                Event::Eof => {
                    // should never happen, as handled in EventList::from_reader()
                    break;
                }
                Event::Start(e) | Event::Empty(e) => {
                    let is_empty = matches!(ev, Event::Empty(_));
                    let mut repeat = 1;
                    let mut event_element = SvgElement::from(e);
                    if let Some(rep_count) = event_element.pop_attr("repeat") {
                        if is_empty {
                            repeat = rep_count.parse().unwrap_or(1);
                        } else {
                            todo!("Repeat is not implemented for non-empty elements");
                        }
                    }
                    for rep_idx in 0..repeat {
                        transform_element(&event_element, &mut self.context, &mut output, is_empty);

                        if rep_idx < (repeat - 1) {
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
                    output.push(&Event::End(BytesEnd::new(ee_name)))
                }
                Event::Text(e) => {
                    // Extract any trailing whitespace following newlines as the current indentation level
                    let re = Regex::new(r"(?ms)\n.*^(\s+)*").unwrap();
                    let text = String::from_utf8(e.to_vec()).expect("Non-UTF8 in input file");
                    if let Some(captures) = re.captures(&text) {
                        let indent = captures
                            .get(1)
                            .map_or(String::from(""), |m| m.as_str().into());
                        self.context.set_indent(indent);
                    }

                    output.push(&Event::Text(e.borrow()))
                }
                _ => {
                    output.push(ev);
                }
            }
        }

        // Calculate bounding box of diagram and use as new viewBox for the image.
        // This also allows just using `<svg>` as the root element.
        // TODO: preserve other attributes from a given svg root.
        let mut extent = BoundingBox::new();
        for el in output.to_elements() {
            if let Some(bb) = &el.bbox() {
                extent.extend(bb);
            }
        }
        // Expand by 10, then add 10%. Ensures small images get more than a couple
        // of pixels border, and large images still get a (relatively) decent border.
        extent.expand(10.);
        extent.scale(1.1);

        if let BoundingBox::BBox(minx, miny, maxx, maxy) = extent {
            let width = fstr(maxx - minx);
            let height = fstr(maxy - miny);
            let mut bs = BytesStart::new("svg");
            bs.push_attribute(Attribute::from(("version", "1.1")));
            bs.push_attribute(Attribute::from(("xmlns", "http://www.w3.org/2000/svg")));
            bs.push_attribute(Attribute::from(("width", format!("{width}mm").as_str())));
            bs.push_attribute(Attribute::from(("height", format!("{height}mm").as_str())));
            bs.push_attribute(Attribute::from((
                "viewBox",
                format!("{} {} {} {}", fstr(minx), fstr(miny), width, height).as_str(),
            )));
            let new_svg = Event::Start(bs);

            for item in &mut output.events.iter_mut() {
                if let Event::Start(x) = item {
                    if x.name().into_inner() == "svg".as_bytes() {
                        *item = new_svg.clone().into_owned();
                        break;
                    }
                }
            }
        }

        output.write_to(writer).map_err(|e| e.to_string())
    }
}

impl From<&BytesStart<'_>> for SvgElement {
    fn from(e: &BytesStart) -> Self {
        let elem_name: String = String::from_utf8(e.name().into_inner().to_vec()).unwrap();

        let attrs: Vec<(String, String)> = e
            .attributes()
            .map(move |a| {
                let aa = a.unwrap();
                let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                let value = aa.unescape_value().unwrap().into_owned();
                (key, value)
            })
            .collect();
        SvgElement::new(&elem_name, &attrs)
    }
}

fn transform_element(
    element: &SvgElement,
    context: &mut TransformerContext,
    output: &mut EventList,
    is_empty: bool,
) {
    let ee = context.handle_element(element, is_empty);
    for svg_ev in ee.into_iter() {
        // re-calculate is_empty for each generated event
        let is_empty = matches!(svg_ev, SvgEvent::Empty(_));
        match svg_ev {
            SvgEvent::Empty(e) | SvgEvent::Start(e) => {
                let mut bs = BytesStart::new(e.name);
                // Collect non-'class' attributes
                for (k, v) in e.attrs.into_iter() {
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
}
