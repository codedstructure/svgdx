use crate::connector::{ConnectionType, Connector};
use crate::expression::eval_attr;
use crate::types::BoundingBox;
use crate::{attr_split_cycle, fstr, strp, strp_length, SvgElement};

use std::collections::HashMap;
use std::io::{BufReader, Read, Write};

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use anyhow::Result;
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

        for ev in events.iter() {
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
                        let aa = a?;

                        let key =
                            String::from_utf8(aa.key.into_inner().to_vec()).expect("not UTF8");
                        let value = aa.unescape_value().expect("XML decode error").into_owned();

                        if &elem_name == "define" {
                            let value = eval_attr(&value, &self.variables);
                            self.variables.insert(key, value);
                        } else if &key == "id" {
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

    fn handle_element(&mut self, e: &SvgElement, empty: bool) -> Result<Vec<SvgEvent>> {
        let elem_name = &e.name;

        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut events = vec![];

        let mut e = e.clone();

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
                d_w = strp_length(&dw);
            }
            if let Some(dh) = dh {
                d_h = strp_length(&dh);
            }
            if d_w.is_some() || d_h.is_some() {
                e = e.resized_by(d_w.unwrap_or_default(), d_h.unwrap_or_default());
                Self::update_elem_map(&mut self.elem_map, &e);
            }
        }

        // "xy-loc" attr allows us to position based on a non-top-left position
        // assumes the bounding-box is well-defined by this point.
        if let (Some(bbox), Some(xy_loc)) = (e.bbox(), e.pop_attr("xy-loc")) {
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
                if e.name == "polyline" {
                    ConnectionType::Corner
                } else if let Some(e_type) = e.get_attr("edge-type") {
                    ConnectionType::from_str(&e_type)
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
                d_x = strp(&dx);
            }
            if let Some(dy) = dy {
                d_y = strp(&dy);
            }
            if d_x.is_some() || d_y.is_some() {
                e = e.translated(d_x.unwrap_or_default(), d_y.unwrap_or_default());
                Self::update_elem_map(&mut self.elem_map, &e);
            }
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
                        events.push(SvgEvent::Text(tt));
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
            e.expand_attributes(true, self)?;
            let new_elem = e.clone();
            if empty {
                events.push(SvgEvent::Empty(new_elem.clone()));
            } else {
                events.push(SvgEvent::Start(new_elem.clone()));
            }
            if new_elem.bbox().is_some() {
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

    fn write_to(&self, writer: &mut dyn Write) -> Result<()> {
        let mut writer = Writer::new(writer);

        for event in &self.events {
            writer.write_event(event)?
        }
        Ok(())
    }

    fn iter(&self) -> impl Iterator<Item = &Event> + '_ {
        self.events.iter()
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

    pub fn transform(&mut self, reader: &mut dyn Read, writer: &mut dyn Write) -> Result<()> {
        let input = EventList::from_reader(reader);
        let mut output = EventList { events: vec![] };

        self.context.populate(&input)?;

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
                        transform_element(
                            &event_element,
                            &mut self.context,
                            &mut output,
                            is_empty,
                        )?;

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
                    output.push(&Event::End(BytesEnd::new(ee_name)));
                }
                Event::Text(e) => {
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
        }

        // Calculate bounding box of diagram and use as new viewBox for the image.
        // This also allows just using `<svg>` as the root element.
        // TODO: preserve other attributes from a given svg root.
        let mut extent = BoundingBox::new();
        let mut elem_path = Vec::new();
        for ev in output.iter() {
            match ev {
                Event::Start(e) | Event::Empty(e) => {
                    let is_empty = matches!(ev, Event::Empty(_));
                    let event_element = SvgElement::from(e);
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
                        if let Some(bb) = &event_element.bbox() {
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
                    if x.name().into_inner() == b"svg" {
                        // }.as_bytes() {
                        *item = new_svg.clone().into_owned();
                        break;
                    }
                }
            }
        }

        output.write_to(writer)
    }
}

impl From<&BytesStart<'_>> for SvgElement {
    fn from(e: &BytesStart) -> Self {
        let elem_name: String =
            String::from_utf8(e.name().into_inner().to_vec()).expect("not UTF8");

        let attrs: Vec<(String, String)> = e
            .attributes()
            .map(move |a| {
                let aa = a.unwrap();
                let key = String::from_utf8(aa.key.into_inner().to_vec()).expect("not UTF8");
                let value = aa.unescape_value().expect("XML decode error").into_owned();
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
) -> Result<()> {
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
    Ok(())
}
