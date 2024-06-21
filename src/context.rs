use crate::connector::{ConnectionType, Connector};
use crate::element::{ContentType, SvgElement};
use crate::events::{InputEvent, SvgEvent};
use crate::expression::eval_attr;
use crate::text::process_text_attr;
use crate::types::{attr_split, attr_split_cycle, strp, BoundingBox, LocSpec, TrblLength};

use std::cell::RefCell;
use std::collections::HashMap;

use anyhow::{bail, Context, Result};

use rand::prelude::*;

pub struct TransformerContext {
    elem_map: HashMap<String, SvgElement>,
    original_map: HashMap<String, SvgElement>,
    current_element: Vec<SvgElement>,
    prev_element: Option<SvgElement>,
    pub variables: HashMap<String, String>,
    // SmallRng is used as it is seedable.
    rng: RefCell<SmallRng>,
    pub real_svg: bool,
    pub in_specs: bool,
    pub events: Vec<InputEvent<'static>>,
}

impl Default for TransformerContext {
    fn default() -> Self {
        Self {
            elem_map: HashMap::new(),
            original_map: HashMap::new(),
            current_element: Vec::new(),
            prev_element: None,
            variables: HashMap::new(),
            rng: RefCell::new(SmallRng::seed_from_u64(0)),
            real_svg: false,
            in_specs: false,
            events: Vec::new(),
        }
    }
}

impl TransformerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_events(&mut self, events: Vec<InputEvent<'static>>) {
        self.events = events;
    }

    pub fn get_element(&self, id: &str) -> Option<&SvgElement> {
        self.elem_map.get(id)
    }

    pub fn get_original_element(&self, id: &str) -> Option<&SvgElement> {
        self.original_map.get(id)
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

    /// Lookup variable in either parent attribute values or global variables
    /// set using the `<var>` element.
    pub fn get_var(&self, name: &str) -> Option<String> {
        // Note we skip the element we're currently processing so we can access
        // variables of the same name, e.g. `<g x="2"/><rect x="$x"/></g>`
        // requires that when evaluating `x="$x"` we don't look up `x` in the
        // `rect` element itself.
        for element_scope in self.current_element.iter().rev().skip(1) {
            if let Some(value) = element_scope.get_attr(name) {
                return Some(value.to_string());
            }
        }
        return self.variables.get(name).cloned();
    }

    pub fn get_prev_element(&self) -> Option<&SvgElement> {
        self.prev_element.as_ref()
    }

    pub fn update_element(&mut self, el: &SvgElement) {
        if let Some(id) = el.get_attr("id") {
            let id = eval_attr(&id, self);
            if self.elem_map.insert(id.clone(), el.clone()).is_none() {
                self.original_map.insert(id, el.clone());
            }
        }
    }

    fn handle_var_element(&mut self, e: &mut SvgElement) {
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
            let margin: TrblLength = margin.parse()?;

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
    pub fn handle_element(&mut self, e: &SvgElement) -> Result<Vec<SvgEvent>> {
        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut events = vec![];

        let mut e = e.clone();

        if &e.name == "var" {
            self.handle_var_element(&mut e);
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
            let xy_loc: LocSpec = xy_loc.as_str().parse().context("Invalid xy-loc value")?;
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
                self.update_element(&e);
            } else {
                bail!("Cannot create connector {e}");
            }
        }

        // Process dx / dy / dxy as translation offsets if not an element
        // where they already have intrinsic meaning.
        if !matches!(e.name.as_str(), "text" | "tspan" | "feOffset") {
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
            if orig_elem.name != "text" {
                // We only care about the original element if it wasn't a text element
                // (otherwise we generate a useless empty text element for the original)
                events.push(SvgEvent::Empty(orig_elem));
                events.push(SvgEvent::Text(format!("\n{}", " ".repeat(e.indent))));
            }
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
