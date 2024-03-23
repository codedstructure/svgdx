use crate::element::ContentType;
use crate::events::{EventList, Index, InputEvent, SvgEvent};
use crate::svg_defs::{build_defs, build_styles};
use crate::types::{fstr, BoundingBox, LocSpec};
use crate::{element::SvgElement, TransformConfig};

use crate::context::TransformerContext;

use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, Write};
use std::ops::RangeBounds;

use itertools::Itertools;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};

use anyhow::{bail, Context, Result};

pub struct Transformer {
    context: TransformerContext,
    config: TransformConfig,
}

trait EventSlice {
    fn range_from(&self, range: impl RangeBounds<Index> + std::fmt::Debug) -> &[InputEvent<'_>];
}

impl<'a> EventSlice for &'a [InputEvent<'_>] {
    fn range_from(&self, range: impl RangeBounds<Index> + std::fmt::Debug) -> &[InputEvent<'_>] {
        // Index isn't an integral type, so we have to traverse the whole list to find the start
        // and end indices. Could be optimized if necessary.
        let start = match range.start_bound() {
            std::ops::Bound::Included(start) => self
                .iter()
                .position(|ev| ev.index == *start)
                .expect("Range should be in slice"),
            std::ops::Bound::Excluded(start) => {
                self.iter()
                    .position(|ev| ev.index == *start)
                    .expect("Range should be in slice")
                    + 1
            }
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(end) => {
                self.iter()
                    .position(|ev| ev.index == *end)
                    .expect("Range should be in slice")
                    + 1
            }
            std::ops::Bound::Excluded(end) => self
                .iter()
                .position(|ev| ev.index == *end)
                .expect("Range should be in slice"),
            std::ops::Bound::Unbounded => self.len(),
        };
        &self[start..end]
    }
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

    fn process_seq(
        &mut self,
        seq: &[InputEvent],
        idx_output: &mut BTreeMap<Index, EventList>,
    ) -> Result<()> {
        let mut last_event = None;
        let mut last_element = None;
        let mut gen_events: Vec<(Index, EventList<'_>)>;

        let mut group_start = None;
        let mut group_depth = 0;

        for input_ev in seq.iter() {
            if input_ev.processed.get() {
                continue;
            }
            let mut processed_flag = true;
            let ev = &input_ev.event;
            gen_events = Vec::new();
            if group_start.is_some() {
                // Skip processing of elements within a group until the group is closed,
                // but need to check that we only start processing again after *this*
                // group has been closed.
                match ev {
                    Event::Start(ref e) => {
                        if e.name().as_ref() == b"g" {
                            group_depth += 1;
                        }
                        continue;
                    }
                    Event::End(e) => {
                        if e.name().as_ref() == b"g" {
                            group_depth -= 1;
                            if group_depth > 0 {
                                continue;
                            }
                        }
                    }
                    _ => {
                        continue;
                    }
                }
            }

            match ev {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    let is_empty = matches!(ev, Event::Empty(_));

                    let mut event_element = SvgElement::try_from(e).context(format!(
                        "could not extract element at line {}",
                        input_ev.line
                    ))?;
                    event_element.set_indent(input_ev.indent);
                    event_element.set_src_line(input_ev.line);
                    event_element.set_order_index(input_ev.index.clone());
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
                        input_ev.processed.set(processed_flag);
                        continue;
                    }

                    if event_element.name == "specs" && !is_empty {
                        if self.context.in_specs {
                            bail!("Cannot nest <specs> elements");
                        }
                        self.context.in_specs = true;
                    }

                    if event_element.name == "g" && group_start.is_none() {
                        group_start = Some(input_ev);
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
                                gen_events.push((input_ev.index.clone(), ev_events.clone()));
                            }
                        } else {
                            processed_flag = false;
                        }

                        self.context.pop_current_element();
                    }
                    input_ev.processed.set(processed_flag);
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

                        if ee_name.as_str() == "g" {
                            // This *should* always be the g element corresponding to group_start,
                            // due to the group_depth check -> continue above.
                            if let Some(start_ev) = group_start {
                                let range = &start_ev.index..&input_ev.index;
                                self.process_seq(seq.range_from(range), idx_output)?;
                                // Important we only mark group start as processed if we didn't error out above
                                start_ev.processed.set(true);
                                group_start = None;
                            }
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
                                gen_events
                                    .push((event_element.order_index.clone(), events.clone()));
                                if !event_element.is_content_text() {
                                    // Similarly, `is_content_text` elements should close themselves in the returned
                                    // event list if needed.
                                    gen_events.push((
                                        input_ev.index.clone(),
                                        EventList::from(ev.clone()),
                                    ));
                                }
                            }
                        } else {
                            processed_flag = false;
                        }
                        last_element = Some(event_element);
                    }
                    input_ev.processed.set(processed_flag);
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
                            input_ev.processed.set(processed_flag);
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
                        gen_events.push((input_ev.index.clone(), EventList::from(ev.clone())));
                    }
                    input_ev.processed.set(true);
                }
                _ => {
                    input_ev.processed.set(true);
                    gen_events.push((input_ev.index.clone(), EventList::from(ev.clone())));
                }
            }

            for (gen_idx, gen_events) in gen_events {
                idx_output.insert(gen_idx, gen_events.into_owned());
            }

            last_event = Some(ev.clone());
        }

        Ok(())
    }

    fn process_events<'a>(&mut self, input: EventList<'a>) -> Result<EventList<'a>> {
        let mut output = Vec::new();
        let mut idx_output = BTreeMap::<Index, EventList>::new();

        let mut last_processed_count = 0;
        loop {
            self.process_seq(input.events.as_ref().as_slice(), &mut idx_output)?;
            let count = input.iter().filter(|ev| ev.processed.get()).count();
            if count == input.len() {
                break;
            } else if count <= last_processed_count {
                bail!(
                    "Could not resolve the following elements:\n{}",
                    input
                        .iter()
                        .filter(|ev| !ev.processed.get())
                        .map(|r| format!("{:4}: {:?}", r.line, r.event))
                        .join("\n")
                );
            }
            last_processed_count = count;
        }

        for (_idx, events) in idx_output {
            output.extend(events.events.iter().cloned().collect::<Vec<_>>());
        }

        Ok(EventList::from(output))
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_process_seq() {
//         let mut transformer = Transformer::from_config(&TransformConfig::default());
//         let mut idx_output = BTreeMap::new();
//         let seq = EventList::new();

//         let result = transformer.process_seq(seq, &mut idx_output);

//         assert!(result.is_ok());
//     }

//     #[test]
//     fn test_process_seq_multiple_elements() {
//         let mut transformer = Transformer::from_config(&TransformConfig::default());
//         let mut idx_output = BTreeMap::new();

//         let seq = EventList::from(
//             r##"<svg>
//           <rect xy="#a:h" wh="10"/>
//           <circle id="a" cx="50" cy="50" r="40"/>
//         </svg>"##,
//         );

//         transformer.process_seq(seq, &mut idx_output).unwrap();
//     }
// }
