use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::io::{BufReader, Write};

use std::collections::{HashMap, HashSet};

use std::fs::File;

use regex::Regex;

fn fstr(x: f32) -> String {
    if x == (x as u32) as f32 {
        return (x as u32).to_string();
    }
    format!("{:.4}", x.to_string())
}

fn strp(s: &str) -> f32 {
    s.parse().unwrap()
}

#[derive(Clone, Debug)]
struct SvgElement {
    name: String,
    attrs: Vec<(String, String)>,
    attr_map: HashMap<String, String>,
    classes: HashSet<String>,
}

impl SvgElement {
    fn new(name: &str, attrs: &[(String, String)]) -> Self {
        let mut attr_map: HashMap<String, String> = HashMap::new();
        let mut classes: HashSet<String> = HashSet::new();

        for (key, value) in attrs {
            if key == "class" {
                for c in value.split(' ') {
                    classes.insert(c.to_string());
                }
            } else {
                attr_map.insert(key.to_string(), value.to_string());
            }
        }
        Self {
            name: name.to_string(),
            attrs: attrs.to_vec(),
            attr_map,
            classes,
        }
    }

    fn add_class(&mut self, class: &str) -> Self {
        self.classes.insert(class.to_string());
        self.clone()
    }

    fn has_attr(&self, key: &str) -> bool {
        self.attr_map.contains_key(key)
    }

    fn with_attr(&self, key: &str, value: &str) -> Self {
        let mut attrs = self.attrs.clone();
        attrs.push((key.into(), value.into()));
        SvgElement::new(self.name.as_str(), &attrs)
    }

    fn without_attr(&self, key: &str) -> Self {
        let attrs: Vec<(String, String)> = self
            .attrs
            .clone()
            .into_iter()
            .filter(|(k, _v)| k != key)
            .collect();
        SvgElement::new(self.name.as_str(), &attrs)
    }

    fn bbox(&self) -> Option<(f32, f32, f32, f32)> {
        match self.name.as_str() {
            "rect" | "tbox" | "pipeline" => {
                let (x, y, w, h) = (
                    strp(self.attr_map.get("x").unwrap()),
                    strp(self.attr_map.get("y").unwrap()),
                    strp(self.attr_map.get("width").unwrap()),
                    strp(self.attr_map.get("height").unwrap()),
                );
                Some((x, y, x + w, y + h))
            }
            "line" => {
                let (x1, y1, x2, y2) = (
                    strp(self.attr_map.get("x1").unwrap()),
                    strp(self.attr_map.get("y1").unwrap()),
                    strp(self.attr_map.get("x2").unwrap()),
                    strp(self.attr_map.get("y2").unwrap()),
                );
                Some((x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)))
            }
            "circle" => {
                let (cx, cy, r) = (
                    strp(self.attr_map.get("cx").unwrap()),
                    strp(self.attr_map.get("cy").unwrap()),
                    strp(self.attr_map.get("r").unwrap()),
                );
                Some((cx - r, cy - r, cx + r, cy + r))
            }
            "person" => {
                let (x, y, h) = (
                    strp(self.attr_map.get("x").unwrap()),
                    strp(self.attr_map.get("y").unwrap()),
                    strp(self.attr_map.get("height").unwrap()),
                );
                Some((x, y, x + h / 3., y + h))
            }

            _ => None,
        }
    }

    fn coord(&self, loc: &str) -> Option<(f32, f32)> {
        // This assumes a rectangular bounding box
        // TODO: support per-shape locs - e.g. "in" / "out" for pipeline
        if let Some((x1, y1, x2, y2)) = self.bbox() {
            match loc {
                "tl" => Some((x1, y1)),
                "t" => Some(((x1 + x2) / 2., y1)),
                "tr" => Some((x2, y1)),
                "r" => Some((x2, (y1 + y2) / 2.)),
                "br" => Some((x2, y2)),
                "b" => Some(((x1 + x2) / 2., y2)),
                "bl" => Some((x1, y2)),
                "l" => Some((x1, (y1 + y2) / 2.)),
                "c" => Some(((x1 + x2) / 2., (y1 + y2) / 2.)),
                _ => None,
            }
        } else {
            None
        }
    }

    fn translated(&self, dx: f32, dy: f32) -> Self {
        let mut attrs = vec![];
        for (key, mut value) in self.attrs.clone() {
            match key.as_str() {
                "x" | "cx" | "x1" | "x2" => {
                    value = fstr(strp(&value) + dx);
                }
                "y" | "cy" | "y1" | "y2" => {
                    value = fstr(strp(&value) + dy);
                }
                _ => (),
            }
            attrs.push((key.clone(), value.clone()));
        }
        SvgElement::new(self.name.as_str(), &attrs)
    }

    fn positioned(&self, x: f32, y: f32) -> Self {
        // TODO: should allow specifying which loc this positions; this currently
        // sets the top-left (for rect/tbox), but for some scenarios it might be
        // necessary to set e.g. the bottom-right loc.
        let mut result = self.clone();
        match self.name.as_str() {
            "rect" | "tbox" | "pipeline" => {
                result = result.with_attr("x", &fstr(x)).with_attr("y", &fstr(y));
            }
            _ => {
                todo!()
            }
        }
        result
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

enum SvgEvent {
    Text(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
}

#[derive(Default)]
struct TransformerContext {
    elem_map: HashMap<String, SvgElement>,
    prev_element: Option<SvgElement>,
    last_indent: String,
}

impl TransformerContext {
    fn new(reader: &mut Reader<BufReader<File>>) -> Self {
        let mut elem_map: HashMap<String, SvgElement> = HashMap::new();
        let mut buf = Vec::new();

        loop {
            let ev = reader.read_event_into(&mut buf);

            match ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let mut attr_map = HashMap::new();
                    let mut attr_list = vec![];
                    let mut id_opt = None;
                    for a in e.attributes() {
                        let aa = a.unwrap();

                        let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                        let value = aa.unescape_value().unwrap().into_owned();

                        if &key == "id" {
                            id_opt = Some(value);
                        } else {
                            attr_map.insert(key.clone(), value.clone());
                            attr_list.push((key, value.clone()));
                        }
                    }
                    if let Some(id) = id_opt {
                        attr_map.insert(
                            String::from("_element_name"),
                            String::from_utf8(e.name().into_inner().to_vec()).unwrap(),
                        );
                        let elem_name: String =
                            String::from_utf8(e.name().into_inner().to_vec()).unwrap();
                        elem_map.insert(id.clone(), SvgElement::new(&elem_name, &attr_list));
                    }
                }
                Ok(_) => {}
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            }
            buf.clear();
        }

        Self {
            elem_map,
            prev_element: None,
            last_indent: String::from(""),
        }
    }

    fn set_indent(&mut self, indent: String) {
        self.last_indent = indent;
    }

    fn eval_ref(&self, attr: &str) -> Option<(f32, f32)> {
        // Example: "#thing@tl" => top left coordinate of element id="thing"
        let re = Regex::new(r"^#(?<id>\w+)(@(?<loc>\S+))?$").unwrap();

        let input = String::from(attr);

        let caps = re.captures(&input)?;
        let name = &caps["id"];
        let loc = caps.name("loc").map_or("c", |v| v.as_str());

        let element = self.elem_map.get(name)?;
        element.coord(loc)
    }

    fn evaluate(&self, input: &str) -> String {
        let re = Regex::new(r"#(\S+):(\S+)").unwrap();

        let mut input = String::from(input);

        // <p id="blob" xy="40 50"/>
        // #blob:xy -> 40 50
        if let Some(caps) = re.captures(&input) {
            let name = caps.get(1).unwrap().as_str();
            let attr = caps.get(2).unwrap().as_str();
            //println!("{} / {}", name, attr);

            if let Some(el) = self.elem_map.get(name) {
                if let Some(value) = el.attr_map.get(attr) {
                    //return value.to_owned();
                    input = value.to_owned();
                }
            }
        }

        input.to_owned()

        // tokenise

        // substitute

        // evaluate
    }

    fn attr_split<'a>(&'a self, input: &'a str) -> impl Iterator<Item = String> + '_ {
        input.split(' ').map(|v| self.evaluate(v))
    }

    fn handle_element(&mut self, e: &SvgElement, empty: bool) -> Vec<SvgEvent> {
        let elem_name = &e.name;
        let mut new_attrs: Vec<(String, String)> = vec![];

        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut new_elems = vec![];

        let mut e = e.clone();

        // Compute any updated geometry from rel attributes
        for (key, value) in &e.attrs.clone() {
            if key.as_str() == "rel" {
                let mut parts = self.attr_split(value);
                if let Some(prev) = &self.prev_element {
                    // Example: rel="tr 2 0"  -> next element top-left starts at prev@tr+(2,0)
                    let loc = parts.next().unwrap();
                    let dx = strp(parts.next().unwrap().as_str());
                    let dy = strp(parts.next().unwrap().as_str());

                    let (prev_x, prev_y) =
                        prev.coord(&loc).expect("Cannot use rel on this element");
                    let rel_x = prev_x + dx;
                    let rel_y = prev_y + dy;
                    e = e.without_attr("rel").positioned(rel_x, rel_y);
                }
            }
        }

        // Process and expand attributes as needed
        for (key, value) in &e.attrs {
            let value = self.evaluate(value.as_str());
            match key.as_str() {
                "xy" => {
                    let mut parts = self.attr_split(&value);

                    match elem_name.as_str() {
                        "rect" | "tbox" | "pipeline" => {
                            new_attrs.push(("x".into(), parts.next().unwrap()));
                            new_attrs.push(("y".into(), parts.next().unwrap()));
                        }
                        "circle" => {
                            new_attrs.push(("cx".into(), parts.next().unwrap()));
                            new_attrs.push(("cy".into(), parts.next().unwrap()));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "size" => {
                    let mut parts = self.attr_split(&value);

                    match elem_name.as_str() {
                        "rect" | "tbox" | "pipeline" => {
                            new_attrs.push(("width".into(), parts.next().unwrap()));
                            new_attrs.push(("height".into(), parts.next().unwrap()));
                        }
                        "circle" => {
                            // TBD: arguably size should map to diameter, for consistency
                            // with width/height on rects - i.e. use 'fstr(w.parse::<f32>*2)'
                            new_attrs.push(("r".into(), parts.next().unwrap()));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "xy1" => match elem_name.as_str() {
                    "line" => {
                        let mut parts = self.attr_split(&value);
                        new_attrs.push(("x1".into(), parts.next().unwrap()));
                        new_attrs.push(("y1".into(), parts.next().unwrap()));
                    }
                    _ => new_attrs.push((key.clone(), value)),
                },
                "start" => match elem_name.as_str() {
                    "line" => {
                        let (start_x, start_y) = self.eval_ref(&value).unwrap();

                        new_attrs.push(("x1".into(), fstr(start_x)));
                        new_attrs.push(("y1".into(), fstr(start_y)));
                    }
                    _ => new_attrs.push((key.clone(), value)),
                },
                "xy2" => match elem_name.as_str() {
                    "line" => {
                        let mut parts = self.attr_split(&value);
                        new_attrs.push(("x2".into(), parts.next().unwrap()));
                        new_attrs.push(("y2".into(), parts.next().unwrap()));
                    }
                    _ => new_attrs.push((key.clone(), value)),
                },
                "end" => match elem_name.as_str() {
                    "line" => {
                        let (end_x, end_y) = self.eval_ref(&value).unwrap();

                        new_attrs.push(("x2".into(), fstr(end_x)));
                        new_attrs.push(("y2".into(), fstr(end_y)));
                    }
                    _ => new_attrs.push((key.clone(), value)),
                },
                _ => new_attrs.push((key.clone(), value)),
            }
        }

        // Expand any custom element types
        match elem_name.as_str() {
            "tbox" => {
                let mut rect_attrs = vec![];
                let mut text_attrs = vec![];

                let mut text = None;

                for (key, value) in &e.attrs {
                    rect_attrs.push((key.clone(), value.clone()));
                    if key == "x" || key == "y" {
                        text_attrs.push((key.clone(), value.clone()));
                    } else if key == "text" {
                        // allows an empty element to contain text content directly as an attribute
                        text = Some(value);
                    }
                }

                let rect_elem = SvgElement::new("rect", &rect_attrs);
                prev_element = Some(rect_elem.clone());
                new_elems.push(SvgEvent::Empty(rect_elem));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Start(SvgElement::new("text", &text_attrs)));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Start(SvgElement::new(
                    "tspan",
                    &[
                        ("dx".to_string(), "2".to_string()),
                        ("dy".to_string(), "6".to_string()),
                    ],
                )));
                // if this *isn't* empty, we'll now expect a text event, which will be passed through.
                // the corresponding </tbox> will be converted into a pair of </tspan> and </text> elements.
                if empty {
                    if let Some(tt) = text {
                        new_elems.push(SvgEvent::Text(format!(
                            "\n{}  {}\n{}",
                            self.last_indent, tt, self.last_indent
                        )));
                        new_elems.push(SvgEvent::End("tspan".to_string()));
                        new_elems.push(SvgEvent::End("text".to_string()));
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

                for (key, value) in &e.attrs {
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

                new_elems.push(SvgEvent::Empty(
                    SvgElement::new("circle", &head_attrs).add_class("person"),
                ));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Empty(
                    SvgElement::new("line", &body_attrs).add_class("person"),
                ));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Empty(
                    SvgElement::new("line", &arms_attrs).add_class("person"),
                ));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Empty(
                    SvgElement::new("line", &leg1_attrs).add_class("person"),
                ));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Empty(
                    SvgElement::new("line", &leg2_attrs).add_class("person"),
                ));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));

                omit = true;
            }
            "pipeline" => {
                let mut x = 0.;
                let mut y = 0.;
                let mut width = 0.;
                let mut height = 0.;
                let mut common_attrs = vec![];
                for (key, value) in &e.attrs {
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
                new_elems.push(SvgEvent::Empty(
                    SvgElement::new("path", &common_attrs).add_class("pipeline"),
                ));
            }
            _ => {}
        }

        if !omit {
            let new_elem = SvgElement::new(elem_name, &new_attrs);
            if empty {
                new_elems.push(SvgEvent::Empty(new_elem.clone()));
            } else {
                new_elems.push(SvgEvent::Start(new_elem.clone()));
            }
            if new_elem.bbox().is_some() {
                // prev_element is only used for relative positioning, so
                // only makes sense if it has a bounding box.
                prev_element = Some(new_elem.clone());
            }
        }
        self.prev_element = prev_element;

        new_elems
    }
}

pub struct Transformer {
    context: TransformerContext,
    reader: Reader<BufReader<File>>,
    writer: Writer<Box<dyn Write>>,
}

impl Transformer {
    pub fn new(filename: &str, output_file_path: &Option<String>) -> Self {
        let mut pre_reader = Reader::from_file(filename).unwrap();
        let reader = Reader::from_file(filename).unwrap();

        let out_writer = match output_file_path {
            Some(x) => {
                let path = std::path::Path::new(x);
                Box::new(File::create(path).unwrap()) as Box<dyn Write>
            }
            None => Box::new(std::io::stdout()) as Box<dyn Write>,
        };

        Self {
            context: TransformerContext::new(&mut pre_reader),
            reader,
            writer: Writer::new(out_writer),
        }
    }

    pub fn transform(&mut self) -> Result<(), String> {
        let mut buf = Vec::new();

        loop {
            let ev = self.reader.read_event_into(&mut buf);

            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let is_empty = matches!(ev, Ok(Event::Empty(_)));
                    let ee = self.context.handle_element(&SvgElement::from(e), is_empty);
                    let mut result = Ok(());
                    for svg_ev in ee.into_iter() {
                        // re-calculate is_empty for each generated event
                        let is_empty = matches!(svg_ev, SvgEvent::Empty(_));
                        match svg_ev {
                            SvgEvent::Empty(e) | SvgEvent::Start(e) => {
                                let mut bs = BytesStart::new(e.name);
                                // Collect non-'class' attributes
                                for (k, v) in e.attrs.into_iter() {
                                    if k != "class" {
                                        bs.push_attribute(Attribute::from((
                                            k.as_bytes(),
                                            v.as_bytes(),
                                        )));
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
                                result = self.writer.write_event(if is_empty {
                                    Event::Empty(bs)
                                } else {
                                    Event::Start(bs)
                                });
                            }
                            SvgEvent::Text(t) => {
                                result = self.writer.write_event(Event::Text(BytesText::new(&t)));
                            }
                            SvgEvent::End(name) => {
                                result = self.writer.write_event(Event::End(BytesEnd::new(name)));
                            }
                        }
                    }
                    result
                }
                Ok(Event::End(e)) => {
                    let mut ee_name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                    if ee_name.as_str() == "tbox" {
                        self.writer.write_event(Event::Text(BytesText::new(&format!(
                            "\n{}",
                            self.context.last_indent
                        ))));
                        self.writer.write_event(Event::End(BytesEnd::new("tspan")));
                        ee_name = String::from("text");
                    }
                    self.writer.write_event(Event::End(BytesEnd::new(ee_name)))
                }
                Ok(Event::Text(e)) => {
                    // Extract any trailing whitespace following newlines as the current indentation level
                    let re = Regex::new(r"(?ms)\n.*^(\s+)*").unwrap();
                    let text = String::from_utf8(e.to_vec()).expect("Non-UTF8 in input file");
                    if let Some(captures) = re.captures(&text) {
                        let indent = captures
                            .get(1)
                            .map_or(String::from(""), |m| m.as_str().into());
                        self.context.set_indent(indent);
                    }

                    self.writer.write_event(Event::Text(e.borrow()))
                }
                Ok(event) => {
                    // This covers XML processing instructions, comments etc
                    self.writer.write_event(event)
                }
                Err(e) => panic!(
                    "Error at position {}: {:?}",
                    self.reader.buffer_position(),
                    e
                ),
            }
            .expect("Failed to parse XML");

            buf.clear();
        }

        Ok(())
    }
}
