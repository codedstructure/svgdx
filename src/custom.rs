use crate::element::SvgElement;
use crate::fstr;
use crate::transform::SvgEvent;

use anyhow::Result;

/// Process non-standard elements.
///
/// Provide the element and a bool indicating whether this is an empty
/// element or not.
/// Returns:
///  - Ok(None) if no custom element was matched
///  - Ok((new_previous_element, vec_of_svgevents_to_append)) if processed.
///    In this case the 'current' element which was passed in should be
///    omitted, replaced with the new set of events returned.
///  - Err(_) if an error occurred
pub fn process_custom(
    element: &SvgElement,
    empty: bool,
) -> Result<Option<(SvgElement, Vec<SvgEvent>)>> {
    // Expand any custom element types
    Ok(match element.name.as_str() {
        "tbox" => {
            let mut events = vec![];
            let mut rect_attrs = vec![];
            let mut text_attrs = vec![];

            let mut text = None;

            for (key, value) in element.clone().attrs {
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
            let cxy = rect_elem.bbox()?.unwrap().center().unwrap();
            text_attrs.push(("x".into(), fstr(cxy.0)));
            text_attrs.push(("y".into(), fstr(cxy.1)));
            let bbox = rect_elem.clone();
            events.push(SvgEvent::Empty(rect_elem));
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
            Some((bbox, events))
        }

        "person" => {
            let mut events = vec![];
            let mut h: f32 = 100.;
            let mut x1: f32 = 0.;
            let mut y1: f32 = 0.;
            let mut common_attrs = vec![];
            let mut head_attrs = vec![];
            let mut body_attrs = vec![];
            let mut arms_attrs = vec![];
            let mut leg1_attrs: Vec<(String, String)> = vec![];
            let mut leg2_attrs: Vec<(String, String)> = vec![];

            for (key, value) in element.clone().attrs {
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
            common_attrs.push(("class".into(), element.classes.to_vec().join(" ")));

            head_attrs.push(("cx".into(), fstr(x1 + h / 6.)));
            head_attrs.push(("cy".into(), fstr(y1 + h / 9.)));
            head_attrs.push(("r".into(), fstr(h / 9.)));
            head_attrs.push(("style".into(), "fill:none; stroke-width:0.5".into()));
            head_attrs.extend(common_attrs.clone());
            let mut head = SvgElement::new("circle", &head_attrs);

            body_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
            body_attrs.push(("y1".into(), fstr(y1 + h / 9. * 2.)));
            body_attrs.push(("x2".into(), fstr(x1 + h / 6.)));
            body_attrs.push(("y2".into(), fstr(y1 + (5.5 * h) / 9.)));
            body_attrs.extend(common_attrs.clone());
            let mut body = SvgElement::new("line", &body_attrs);

            arms_attrs.push(("x1".into(), fstr(x1)));
            arms_attrs.push(("y1".into(), fstr(y1 + h / 3.)));
            arms_attrs.push(("x2".into(), fstr(x1 + h / 3.)));
            arms_attrs.push(("y2".into(), fstr(y1 + h / 3.)));
            arms_attrs.extend(common_attrs.clone());
            let mut arms = SvgElement::new("line", &arms_attrs);

            leg1_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
            leg1_attrs.push(("y1".into(), fstr(y1 + (5.5 * h) / 9.)));
            leg1_attrs.push(("x2".into(), fstr(x1)));
            leg1_attrs.push(("y2".into(), fstr(y1 + h)));
            leg1_attrs.extend(common_attrs.clone());
            let mut leg1 = SvgElement::new("line", &leg1_attrs);

            leg2_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
            leg2_attrs.push(("y1".into(), fstr(y1 + (5.5 * h) / 9.)));
            leg2_attrs.push(("x2".into(), fstr(x1 + h / 3.)));
            leg2_attrs.push(("y2".into(), fstr(y1 + h)));
            leg2_attrs.extend(common_attrs.clone());
            let mut leg2 = SvgElement::new("line", &leg2_attrs);

            events.push(SvgEvent::Empty(head.add_class("person")));
            events.push(SvgEvent::Empty(body.add_class("person")));
            events.push(SvgEvent::Empty(arms.add_class("person")));
            events.push(SvgEvent::Empty(leg1.add_class("person")));
            events.push(SvgEvent::Empty(leg2.add_class("person")));

            // Since we're omitting the original element we need to set a separate
            // element to act as the previous element for relative positioning
            let bbox = SvgElement::new(
                "rect",
                &[
                    ("x".into(), fstr(arms.coord("l")?.unwrap().0)),
                    ("y".into(), fstr(head.coord("t")?.unwrap().1)),
                    (
                        "width".into(),
                        fstr(arms.coord("r")?.unwrap().0 - arms.coord("l")?.unwrap().0),
                    ),
                    (
                        "height".into(),
                        fstr(leg1.coord("b")?.unwrap().1 - head.coord("t")?.unwrap().1),
                    ),
                ],
            );
            Some((bbox, events))
        }
        "pipeline" => {
            let mut events = vec![];
            let mut x = 0.;
            let mut y = 0.;
            let mut width = 0.;
            let mut height = 0.;
            let mut common_attrs = vec![];
            for (key, value) in element.clone().attrs {
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
            common_attrs.push(("class".into(), element.classes.to_vec().join(" ")));

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

            Some((bbox, events))
        }
        _ => None,
    })
}
