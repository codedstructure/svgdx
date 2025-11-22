use super::SvgElement;
use crate::errors::{Error, Result};
use crate::types::{attr_split, fstr, strp};

pub fn points_to_path(element: &SvgElement) -> Result<SvgElement> {
    let (mut points, max_radius) = if let (Some(r), Some(p)) = (
        element.get_attr("corner-radius"),
        element.get_attr("points"),
    ) {
        let floats: Vec<f32> = attr_split(p).filter_map(|a| strp(&a).ok()).collect();
        // chunks_exact to ignore any unpaired final number
        (
            floats
                .chunks_exact(2)
                .map(|a| (a[0], a[1]))
                .collect::<Vec<_>>(),
            strp(r)?,
        )
    } else {
        return Err(Error::InternalLogic(
            "points_to_path() needs points and corner-radius".to_string(),
        ));
    };

    let polygon = element.name() == "polygon";

    let mut result = vec![];
    if points.is_empty() {
        result.push(String::new());
    } else {
        let mut points_no_dupe = vec![];
        let first_item = points[0];
        for p in 0..points.len() {
            if points[p] != points[(p + 1) % points.len()] || (!polygon && p == points.len() - 1) {
                points_no_dupe.push(points[p]);
            }
        }
        points = points_no_dupe;

        if points.len() <= 1 {
            result.push(format!(
                "M {} {} l 0 0",
                fstr(first_item.0),
                fstr(first_item.1)
            ));
        } else {
            result = points_to_path_render(&points, polygon, max_radius);
        }
    }

    // create new element and copy attrs and replace points with path attr
    let mut new_element = SvgElement::new("path", &[]);
    new_element = new_element.with_attrs_from(element);

    new_element.pop_attr("points");
    new_element.set_attr("d", &result.join(" "));

    Ok(new_element)
}

fn points_to_path_render(points: &[(f32, f32)], polygon: bool, max_radius: f32) -> Vec<String> {
    // radii coresponding to each corner
    // may be smaller than max as if 2 adjacent points are too close
    // then their curves would overlap resulting in obvious error
    // decided to limit to half distance to closest neighbour as simple and often good enough
    let mut radii = vec![];
    for i in 1..(points.len() - 1) {
        //           v current point considering
        // x---------x------x
        //  <--d1---> <-d2->
        let mut d1 = (points[i].0 - points[i - 1].0).hypot(points[i].1 - points[i - 1].1);
        let mut d2 = (points[i + 1].0 - points[i].0).hypot(points[i + 1].1 - points[i].1);

        // if it is not a polygon then end points have no radius so dont need to share
        // so dont half
        if i != 1 || polygon {
            d1 /= 2.0;
        }
        if i != points.len() - 2 || polygon {
            d2 /= 2.0;
        }
        let radius = d1.min(d2).min(max_radius);
        radii.push(radius);
    }

    // inited now because may be changed by polygon condition
    let mut pos = points[0];

    // if polygon need to add 2 more corners for each end point and join them up
    if polygon {
        let last = points.len() - 1;
        // v penultimate v last  v first    v second
        // x-------------x-------x----------x
        //  <-----d1----> <--d2-> <---d3--->

        // d1 d2 and d3 are all halved instantly as this is a polygon so nothing special about any point
        let d1 =
            (points[last].0 - points[last - 1].0).hypot(points[last].1 - points[last - 1].1) / 2.0;
        let d2 = (points[0].0 - points[last].0).hypot(points[0].1 - points[last].1) / 2.0;
        let d3 = (points[1].0 - points[0].0).hypot(points[1].1 - points[0].1) / 2.0;
        let radius = d1.min(d2).min(max_radius);
        radii.push(radius);
        let radius = d2.min(d3).min(max_radius);
        radii.push(radius);

        // move by distance of the radius corresponding to first point along d3
        let dx = points[1].0 - pos.0;
        let dy = points[1].1 - pos.1;

        let len = dx.hypot(dy);

        pos.0 += dx * radii[radii.len() - 1] / len;
        pos.1 += dy * radii[radii.len() - 1] / len;
    }

    let mut result = points_to_path_draw_loop(&mut pos, points, radii);
    if !polygon {
        // move to the last point
        pos = points[points.len() - 1];
        result.push(format!("L {} {}", fstr(pos.0), fstr(pos.1)));
    } else {
        // close polygon as even if in same place may look different
        result.push("z".to_string());
    }

    result
}

fn points_to_path_draw_loop(
    pos: &mut (f32, f32),
    points: &[(f32, f32)],
    radii: Vec<f32>,
) -> Vec<String> {
    let mut result = vec![];

    // move to start
    result.push(format!("M {} {}", fstr(pos.0), fstr(pos.1)));

    for i in 0..radii.len() {
        let p1 = points[(i + 1) % points.len()];
        let p2 = points[(i + 2) % points.len()];

        // 1 is from pos to this point
        // 2 is from this point to next point
        let dx1 = p1.0 - pos.0;
        let dy1 = p1.1 - pos.1;
        let dx2 = p2.0 - p1.0;
        let dy2 = p2.1 - p1.1;

        let len1 = dx1.hypot(dy1);
        let len2 = dx2.hypot(dy2);

        // calculate where curve starts
        pos.0 += dx1 - dx1 * radii[i] / len1;
        pos.1 += dy1 - dy1 * radii[i] / len1;

        // move there
        result.push(format!("L {} {}", fstr(pos.0), fstr(pos.1)));

        let mut new_pos = p1;

        // calculate where curve ends
        new_pos.0 += dx2 * radii[i] / len2;
        new_pos.1 += dy2 * radii[i] / len2;

        // using the dot product normalised
        // negated as one line goes into point
        // and one line goes out
        let cos = -(dx1 * dx2 + dy1 * dy2) as f64 / (len1 * len2) as f64;
        // if cos ~~ -1.0 then it is a straight line the corresponding radius is inf or very large
        // to avoid fp errors if corresponding t value > 2000 it is unlikly to work
        // also need to check for greater than 1 due to more fp errors as that does not make sense
        // greater than 1 still semanticly means straight line so same logic is used
        if (cos + 1.0).abs() <= 0.0001 || cos >= 1.0 {
            // for tidyness
            if *pos != new_pos {
                result.push(format!("L {} {}", fstr(new_pos.0), fstr(new_pos.1)));
            }
        } else {
            // this is using a t formulae
            // t = tan(theta/2)
            // cos(theta) = (1-t^2)/(1+t^2)
            // rearange to get t (valid for 0<=theta<pi)
            let t = ((1.0 - cos) / (1.0 + cos)).sqrt();
            // t scales the radius to get used radius
            // draw a kite with 2 corners being right-angles
            // 2 sides are equal to this radius = r
            // the 2 other sides are used radius = a
            // split the kite along diagonal so 2 right-angled triangles
            // it can be seen a = r*t if theta = angle between the 2 rs

            // whether it is going clockwise
            // calculated by taking dotproduct of d1 and (d2 rotated 90deg)
            // equivalent to '2d' cross product
            // the sign of the answer is which way it goes
            let cl = (dx1 * dy2 - dy1 * dx2) > 0.0;

            // the first 0 is rotation and could be any float parsable value
            // the second 0 is whether to do a large arc but for this we never do
            if radii[i] != 0.0 {
                result.push(format!(
                    "a {} {} 0 0 {} {} {}",
                    fstr(radii[i] * t as f32), // radius x
                    fstr(radii[i] * t as f32), // radius y
                    cl as u32,                 // clockwise
                    fstr(new_pos.0 - pos.0),   // dx
                    fstr(new_pos.1 - pos.1),   // dy
                ));
            }
        }

        *pos = new_pos;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_points_to_path_squiggle() {
        let squiggle = &[(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (10.0, 5.0)];

        for (cr, expected) in [
            (
                0.0,
                vec![
                    "M 0 0".to_string(),
                    "L 5 0".to_string(),
                    "L 5 5".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
            (
                1.0,
                vec![
                    "M 0 0".to_string(),
                    "L 4 0".to_string(),
                    "a 1 1 0 0 1 1 1".to_string(),
                    "L 5 4".to_string(),
                    "a 1 1 0 0 0 1 1".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
            (
                5.0,
                vec![
                    "M 0 0".to_string(),
                    "L 2.5 0".to_string(),
                    "a 2.5 2.5 0 0 1 2.5 2.5".to_string(),
                    "L 5 2.5".to_string(),
                    "a 2.5 2.5 0 0 0 2.5 2.5".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
            (
                10.0,
                vec![
                    "M 0 0".to_string(),
                    "L 2.5 0".to_string(),
                    "a 2.5 2.5 0 0 1 2.5 2.5".to_string(),
                    "L 5 2.5".to_string(),
                    "a 2.5 2.5 0 0 0 2.5 2.5".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
        ] {
            let v = points_to_path_render(squiggle, false, cr);
            assert_eq!(v, expected);
        }
    }

    #[test]
    fn test_points_to_path_acute() {
        let acute = &[(0.0, 0.0), (5.0, 0.0), (0.0, 5.0)];
        let v = points_to_path_render(acute, false, 0.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 5 0".to_string(),
                "L 0 5".to_string(),
            ]
        );
        let v = points_to_path_render(acute, false, 2.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 3 0".to_string(),
                "a 0.828 0.828 0 0 1 0.586 1.414".to_string(),
                "L 0 5".to_string(),
            ]
        );
    }

    #[test]
    fn test_points_to_path_obtuse() {
        let obtuse = &[(0.0, 0.0), (5.0, 0.0), (10.0, 5.0)];
        let v = points_to_path_render(obtuse, false, 0.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 5 0".to_string(),
                "L 10 5".to_string(),
            ]
        );
        let v = points_to_path_render(obtuse, false, 2.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 3 0".to_string(),
                "a 4.828 4.828 0 0 1 3.414 1.414".to_string(),
                "L 10 5".to_string(),
            ]
        );
    }

    #[test]
    fn test_points_to_path_polygon() {
        let square = &[(0.0, 0.0), (7.0, 0.0), (7.0, 7.0), (0.0, 7.0)];
        let v = points_to_path_render(square, true, 0.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 7 0".to_string(),
                "L 7 7".to_string(),
                "L 0 7".to_string(),
                "L 0 0".to_string(),
                "z".to_string(),
            ]
        );
        let v = points_to_path_render(square, true, 1.0);
        assert_eq!(
            v,
            [
                "M 1 0".to_string(),
                "L 6 0".to_string(),
                "a 1 1 0 0 1 1 1".to_string(),
                "L 7 6".to_string(),
                "a 1 1 0 0 1 -1 1".to_string(),
                "L 1 7".to_string(),
                "a 1 1 0 0 1 -1 -1".to_string(),
                "L 0 1".to_string(),
                "a 1 1 0 0 1 1 -1".to_string(),
                "z".to_string(),
            ]
        );
    }
}
