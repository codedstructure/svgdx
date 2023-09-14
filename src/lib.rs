use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::io::{BufReader, BufWriter};

use std::collections::HashMap;

use std::fs::File;

use regex::Regex;

fn fstr(x: f32) -> String {
    if x == (x as u32) as f32 {
        return (x as u32).to_string();
    }
    x.to_string()
}

struct SvgElement {
    name: String,
    attrs: Vec<(String, String)>,
}

impl SvgElement {
    fn new(name: &str, attrs: &[(String, String)]) -> Self {
        Self {
            name: name.to_string(),
            attrs: attrs.to_vec(),
        }
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
    vars: HashMap<String, HashMap<String, String>>,
    last_indent: String,
}

impl TransformerContext {
    fn new(reader: &mut Reader<BufReader<File>>) -> Self {
        let mut vars: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut buf = Vec::new();

        loop {
            let ev = reader.read_event_into(&mut buf);

            match ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let mut attr_map = HashMap::new();
                    let mut id_opt = None;
                    for a in e.attributes() {
                        let aa = a.unwrap();

                        let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                        let value = aa.unescape_value().unwrap().into_owned();

                        if &key == "id" {
                            id_opt = Some(value);
                        } else {
                            attr_map.insert(key.clone(), value);
                        }
                    }
                    if let Some(id) = id_opt {
                        attr_map.insert(
                            String::from("_element_name"),
                            String::from_utf8(e.name().into_inner().to_vec()).unwrap(),
                        );
                        vars.insert(id, attr_map);
                    }
                }
                Ok(_) => {}
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            }
            buf.clear();
        }

        println!("CONTEXT: {:?}", vars);

        Self {
            vars,
            last_indent: String::from(""),
        }
    }

    fn set_indent(&mut self, indent: String) {
        self.last_indent = indent;
    }

    fn attr_map(&self, element: &BytesStart) -> Vec<(String, String)> {
        element
            .attributes()
            .filter_map(|s| s.ok())
            .map(|s| {
                (
                    String::from_utf8(s.key.into_inner().to_vec()).unwrap(),
                    s.unescape_value().unwrap().into_owned(),
                )
            })
            .collect()
    }

    fn center(&self, element_id: &str) -> (String, String) {
        let mut cx = String::from("0");
        let mut cy = String::from("0");

        let attrs = self.vars.get(element_id).unwrap();

        let elem_name = attrs.get("_element_name").unwrap();

        match elem_name.as_str() {
            "rect" | "tbox" => {
                let x: f32 = attrs.get("x").unwrap().to_string().parse().unwrap();
                let y: f32 = attrs.get("y").unwrap().to_string().parse().unwrap();
                let w: f32 = attrs.get("width").unwrap().to_string().parse().unwrap();
                let h: f32 = attrs.get("height").unwrap().to_string().parse().unwrap();

                cx = fstr(x + w / 2.);
                cy = fstr(y + h / 2.);
            }
            "circle" => {
                cx = attrs.get("cx").unwrap().to_string();
                cy = attrs.get("cy").unwrap().to_string();
            }
            _ => {
                todo!("Implement...");
            }
        }

        (cx, cy)
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

            if let Some(attr_map) = self.vars.get(name) {
                if let Some(value) = attr_map.get(attr) {
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

    fn split_pair(&self, input: &str) -> (String, Option<String>) {
        let mut in_iter = input.split(' ');
        let a: String = self.evaluate(in_iter.next().expect("Empty"));
        let b: Option<String> = in_iter.next().map(|x| self.evaluate(x));

        (a, b)
    }

    fn handle_element(&self, e: &BytesStart, empty: bool) -> Vec<SvgEvent> {
        let elem_name: String = String::from_utf8(e.name().into_inner().to_vec()).unwrap();
        let mut new_attrs: Vec<(String, String)> = vec![];

        let mut omit = false;
        let mut new_elems = vec![];

        match elem_name.as_str() {
            "tbox" => {
                let mut rect_attrs = vec![];
                let mut text_attrs = vec![];

                let mut text = None;

                for a in e.attributes() {
                    let aa = a.unwrap();

                    let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                    let value = aa.unescape_value().unwrap().into_owned();

                    rect_attrs.push((key.clone(), value.clone()));
                    if key == "x" || key == "y" {
                        text_attrs.push((key, value));
                    } else if key == "text" {
                        // allows an empty element to contain text content directly as an attribute
                        text = Some(value);
                    }
                }

                new_elems.push(SvgEvent::Empty(SvgElement::new("rect", &rect_attrs)));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Start(SvgElement::new("text", &text_attrs)));
                new_elems.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                new_elems.push(SvgEvent::Start(SvgElement::new(
                    "tspan",
                    &[
                        ("dx".to_string(), "1".to_string()),
                        ("dy".to_string(), "8".to_string()),
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
            _ => {}
        }

        for a in e.attributes() {
            let aa = a.unwrap();

            let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
            let value = aa.unescape_value().unwrap().into_owned();

            let value = self.evaluate(value.as_str());

            match key.as_str() {
                "xy" => {
                    let (x, opt_y) = self.split_pair(&value);
                    let y = opt_y.unwrap();

                    match elem_name.as_str() {
                        "rect" => {
                            new_attrs.push(("x".into(), x));
                            new_attrs.push(("y".into(), y));
                        }
                        "circle" => {
                            new_attrs.push(("cx".into(), x));
                            new_attrs.push(("cy".into(), y));
                        }
                        _ => new_attrs.push((key, value)),
                    }
                }
                "size" => {
                    let (w, h_opt) = self.split_pair(&value);

                    match elem_name.as_str() {
                        "rect" => {
                            let h = h_opt.unwrap();
                            new_attrs.push(("width".into(), w));
                            new_attrs.push(("height".into(), h));
                        }
                        "circle" => {
                            // TBD: arguably size should map to diameter, for consistency
                            // with width/height on rects - i.e. use 'fstr(w.parse::<f32>*2)'
                            new_attrs.push(("r".into(), w));
                        }
                        _ => new_attrs.push((key, value)),
                    }
                }
                "xy1" => match elem_name.as_str() {
                    "line" => {
                        let (x, opt_y) = self.split_pair(&value);
                        let y = opt_y.unwrap();
                        new_attrs.push(("x1".into(), x));
                        new_attrs.push(("y1".into(), y));
                    }
                    _ => new_attrs.push((key, value)),
                },
                "start" => match elem_name.as_str() {
                    "line" => {
                        let (start_center_x, start_center_y) = self.center(&value);

                        new_attrs.push(("x1".into(), start_center_x));
                        new_attrs.push(("y1".into(), start_center_y));
                    }
                    _ => new_attrs.push((key, value)),
                },
                "xy2" => match elem_name.as_str() {
                    "line" => {
                        let (x, opt_y) = self.split_pair(&value);
                        let y = opt_y.unwrap();
                        new_attrs.push(("x2".into(), x));
                        new_attrs.push(("y2".into(), y));
                    }
                    _ => new_attrs.push((key, value)),
                },
                "end" => match elem_name.as_str() {
                    "line" => {
                        let (end_center_x, end_center_y) = self.center(&value);

                        new_attrs.push(("x2".into(), end_center_x));
                        new_attrs.push(("y2".into(), end_center_y));
                    }
                    _ => new_attrs.push((key, value)),
                },
                _ => new_attrs.push((key, value)),
            }
        }

        if !omit {
            if empty {
                new_elems.push(SvgEvent::Empty(SvgElement {
                    name: elem_name,
                    attrs: new_attrs,
                }));
            } else {
                new_elems.push(SvgEvent::Start(SvgElement {
                    name: elem_name,
                    attrs: new_attrs,
                }));
            }
        }
        new_elems
    }
}

pub struct Transformer {
    context: TransformerContext,
    reader: Reader<BufReader<File>>,
    writer: Writer<BufWriter<File>>,
}

impl Transformer {
    pub fn new(filename: &str) -> Self {
        let mut pre_reader = Reader::from_file(filename).unwrap();
        let reader = Reader::from_file(filename).unwrap();
        Self {
            context: TransformerContext::new(&mut pre_reader),
            reader,
            writer: Writer::new(BufWriter::new(File::create("out.svg").unwrap())),
        }
    }

    pub fn transform(&mut self) -> Result<(), String> {
        let mut buf = Vec::new();

        loop {
            let ev = self.reader.read_event_into(&mut buf);

            match ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(Event::Start(e)) => {
                    let ee = self.context.handle_element(&e, false);
                    let mut result = Ok(());
                    for ev in ee.into_iter() {
                        match ev {
                            SvgEvent::Empty(e) => {
                                let mut bs = BytesStart::new(e.name);
                                for (k, v) in e.attrs.into_iter() {
                                    bs.push_attribute(Attribute::from((
                                        k.as_bytes(),
                                        v.as_bytes(),
                                    )));
                                }
                                result = self.writer.write_event(Event::Empty(bs));
                            }
                            SvgEvent::Start(e) => {
                                let mut bs = BytesStart::new(e.name);
                                for (k, v) in e.attrs.into_iter() {
                                    bs.push_attribute(Attribute::from((
                                        k.as_bytes(),
                                        v.as_bytes(),
                                    )));
                                }
                                result = self.writer.write_event(Event::Start(bs));
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
                Ok(Event::Empty(e)) => {
                    let ee = self.context.handle_element(&e, true);
                    let mut result = Ok(());
                    for ev in ee.into_iter() {
                        match ev {
                            SvgEvent::Empty(e) => {
                                let mut bs = BytesStart::new(e.name);
                                for (k, v) in e.attrs.into_iter() {
                                    bs.push_attribute(Attribute::from((
                                        k.as_bytes(),
                                        v.as_bytes(),
                                    )));
                                }
                                result = self.writer.write_event(Event::Empty(bs));
                            }
                            SvgEvent::Start(e) => {
                                let mut bs = BytesStart::new(e.name);
                                for (k, v) in e.attrs.into_iter() {
                                    bs.push_attribute(Attribute::from((
                                        k.as_bytes(),
                                        v.as_bytes(),
                                    )));
                                }
                                result = self.writer.write_event(Event::Start(bs));
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

                    self.writer.write_event(Event::Text(e))
                }
                Ok(event) => {
                    //println!("EVENT: {:?}", event);
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
