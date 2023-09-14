use xmltree::{Element, EmitterConfig};

use std::collections::HashMap;

use std::fs;
use std::fs::File;
use std::io;

use regex::Regex;

fn fstr(x: f32) -> String {
    if x == (x as u32) as f32 {
        return (x as u32).to_string();
    }
    return x.to_string();
}


#[derive(Clone)]
pub struct Transformer {
    original: Element,
    root: Element,
}

impl Transformer {
    pub fn new(filename: &str) -> Self {
        let svg_in: String = fs::read_to_string(filename)
            .map_err(|e| e.to_string())
            .unwrap();

        let original = Element::parse(svg_in.as_bytes()).unwrap();
        let root = original.clone();
        Self { original, root }
    }

    fn evaluate(&self, input: &str) -> String {
        let re = Regex::new(r"\$(\S+):(\S+)$").unwrap();

        // <p id="blob" xy="40 50"/>
        // $blob:xy -> 40 50
        if let Some(caps) = re.captures(input) {
            let name = caps.get(1).unwrap().as_str();
            let attr = caps.get(2).unwrap().as_str();
            println!("{} / {}", name, attr);
        }

        input.to_owned()

        // tokenise

        // substiture

        // evaluate
    }

    fn split_pair(&self, input: &str) -> (String, Option<String>) {
        let mut in_iter = input.split(" ");
        let a: String = self.evaluate(in_iter.next().expect("Empty"));
        //.parse()
        //.expect("Invalid pair attribute");
        let b: Option<String> = in_iter.next().and_then(|x| Some(self.evaluate(x)));

        (a, b)
    }

    fn attr_update(&self, element: &Element) -> (HashMap<&str, String>, Vec<&str>) {

        let mut to_add = HashMap::new();
        let mut to_remove = vec![];

        'xy: {
            if let Some(xy) = element.attributes.get("xy") {
                to_remove.push("xy");
                let (x, opt_y) = self.split_pair(xy);
                let y = opt_y.unwrap();

                //element.attributes.remove("xy");
                let x_target;
                let y_target;
                match element.name.as_str() {
                    "rect" => {
                        x_target = "x";
                        y_target = "y";
                    }
                    "circle" => {
                        x_target = "cx";
                        y_target = "cy";
                    }
                    _ => break 'xy,
                }
                to_add.insert(x_target.into(), x);
                to_add.insert(y_target.into(), y);
                //element.attributes.insert(x_target.into(), x);
                //element.attributes.insert(y_target.into(), y);
            }
        }
        'size: {
            if let Some(size) = element.attributes.get("size") {
                let (w, h_opt) = self.split_pair(size);
                to_remove.push("size");
                //element.attributes.remove("size");
                match element.name.as_str() {
                    "rect" => {
                        let h = h_opt.unwrap();
                        to_add.insert("width", w);
                        to_add.insert("height", h);
                        //element.attributes.insert("width".into(), w);
                        //element.attributes.insert("height".into(), h);
                    }
                    "circle" => {
                        to_add.insert("r", w);
                        // element.attributes.insert("r".into(), w);
                    }
                    _ => break 'size,
                }
            }
        }

        (to_add, to_remove)
    }

    fn build_elements<'a>(root: &'a mut Element, accum: &'a mut Vec<&'a mut Element>) {
        for child in root.children.iter_mut() {
            if let Some(el) = child.as_mut_element() {
                accum.push(el);
                Self::build_elements(el, accum);
            }
        }
    }

    pub fn transform(&mut self) -> Result<(), String> {
        let mut variables = HashMap::new();

        // extract variables
        for child in self.root.children.iter_mut() {
            if let Some(el) = child.as_mut_element() {
                if el.name == "p" {
                    variables.insert(
                        el.attributes.get("id").unwrap(),
                        el.attributes.get("value").unwrap(),
                    );
                }
            }
        }

        // replace attributes with final values
        let original = self.clone();
        for child in self.root.children.iter_mut() {
            let to_add;
            let to_remove;
            if let Some(el) = child.as_mut_element() {
                (to_add, to_remove) = original.attr_update(el);
                for attr in to_remove.into_iter() {
                    el.attributes.remove(attr);
                }
                for (attr, value) in to_add.into_iter() {
                    el.attributes.insert(attr.into(), value);
                }
            }
        }

        let config = EmitterConfig::new().perform_indent(true);
        self.root
            .write_with_config(io::stdout(), config)
            .map_err(|e| e.to_string());

        self.root
            .write(File::create("out.svg").unwrap())
            .map_err(|e| e.to_string())
    }
}
