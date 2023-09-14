//use xmltree::{Element, EmitterConfig};

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
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

#[derive(Default)]
struct TransformerContext {
    vars: HashMap<String, HashMap<String, String>>,
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
                        vars.insert(id, attr_map);
                    }
                }
                Ok(_) => {}
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            }
            buf.clear();
        }

        println!("CONTEXT: {:?}", vars);

        Self { vars }
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
        let b: Option<String> = in_iter.next().and_then(|x| Some(self.evaluate(x)));

        (a, b)
    }

    fn handle_element<'a>(&self, e: &'a BytesStart) -> BytesStart<'a> {
        let elem_name: String = String::from_utf8(e.name().into_inner().to_vec()).unwrap();
        let mut new_elem = BytesStart::new(elem_name.clone());
        let mut new_attrs: Vec<(String, String)> = vec![];

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
                "xy1" | "start" => {
                    match elem_name.as_str() {
                        "line" => {
                            let (x, opt_y) = self.split_pair(&value);
                            let y = opt_y.unwrap();
                            new_attrs.push(("x1".into(), x));
                            new_attrs.push(("y1".into(), y));
                        }
                        _ => new_attrs.push((key, value)),
                    }
                }
                "xy2" | "end" => {
                    match elem_name.as_str() {
                        "line" => {
                            let (x, opt_y) = self.split_pair(&value);
                            let y = opt_y.unwrap();
                            new_attrs.push(("x2".into(), x));
                            new_attrs.push(("y2".into(), y));
                        }
                        _ => new_attrs.push((key, value)),
                    }
                }
                _ => new_attrs.push((key, value)),
            }
        }

        for (new_k, new_v) in new_attrs.into_iter() {
            new_elem.push_attribute(Attribute::from((new_k.as_bytes(), new_v.as_bytes())));
        }
        new_elem
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
                    let ee = self.context.handle_element(&e);
                    self.writer.write_event(Event::Start(ee))
                }
                Ok(Event::Empty(e)) => {
                    let ee = self.context.handle_element(&e);
                    self.writer.write_event(Event::Empty(ee))
                }
                Ok(Event::End(e)) => self.writer.write_event(Event::End(e)),
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
