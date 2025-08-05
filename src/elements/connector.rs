use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::SvgElement;
use crate::context::ElementMap;
use crate::errors::{Result, SvgdxError};
use crate::geometry::{parse_el_loc, strp_length, BoundingBox, Length, LocSpec, ScalarSpec};
use crate::types::{attr_split, fstr, strp};

pub fn is_connector(el: &SvgElement) -> bool {
    el.has_attr("start") && el.has_attr("end") && (el.name() == "line" || el.name() == "polyline")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
struct Endpoint {
    origin: (f32, f32),
    dir: Option<Direction>,
}

impl Endpoint {
    const fn new(origin: (f32, f32), dir: Option<Direction>) -> Self {
        Self { origin, dir }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectionType {
    Horizontal,
    Vertical,
    Corner,
    Straight,
}

impl ConnectionType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "h" | "horizontal" => Self::Horizontal,
            "v" | "vertical" => Self::Vertical,
            _ => Self::Straight,
        }
    }
}

fn edge_locations(ctype: ConnectionType) -> Vec<LocSpec> {
    match ctype {
        ConnectionType::Horizontal => vec![LocSpec::Left, LocSpec::Right],
        ConnectionType::Vertical => vec![LocSpec::Top, LocSpec::Bottom],
        ConnectionType::Corner => {
            vec![LocSpec::Top, LocSpec::Right, LocSpec::Bottom, LocSpec::Left]
        }
        ConnectionType::Straight => vec![
            LocSpec::Top,
            LocSpec::Bottom,
            LocSpec::Left,
            LocSpec::Right,
            LocSpec::TopLeft,
            LocSpec::BottomLeft,
            LocSpec::TopRight,
            LocSpec::BottomRight,
        ],
    }
}

#[derive(Clone)]
pub struct Connector {
    source_element: SvgElement,
    start_el: Option<SvgElement>,
    end_el: Option<SvgElement>,
    start: Endpoint,
    end: Endpoint,
    conn_type: ConnectionType,
    offset: Option<Length>,
    corner_radius: f32,
}

fn closest_loc(
    this: &SvgElement,
    point: (f32, f32),
    conn_type: ConnectionType,
    context: &impl ElementMap,
) -> Result<LocSpec> {
    let mut min_dist_sq = f32::MAX;
    let mut min_loc = LocSpec::Center;

    let this_bb = context
        .get_element_bbox(this)?
        .ok_or_else(|| SvgdxError::MissingBoundingBox(this.to_string()))?;

    for loc in edge_locations(conn_type) {
        let this_coord = this_bb.locspec(loc);
        let ((x1, y1), (x2, y2)) = (this_coord, point);
        let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            min_loc = loc;
        }
    }
    Ok(min_loc)
}

fn shortest_link(
    this: &SvgElement,
    that: &SvgElement,
    conn_type: ConnectionType,
    context: &impl ElementMap,
) -> Result<(LocSpec, LocSpec)> {
    let mut min_dist_sq = f32::MAX;
    let mut this_min_loc = LocSpec::Center;
    let mut that_min_loc = LocSpec::Center;

    let this_bb = context
        .get_element_bbox(this)?
        .ok_or_else(|| SvgdxError::MissingBoundingBox(this.to_string()))?;
    let that_bb = context
        .get_element_bbox(that)?
        .ok_or_else(|| SvgdxError::MissingBoundingBox(that.to_string()))?;

    for this_loc in edge_locations(conn_type) {
        for that_loc in edge_locations(conn_type) {
            let this_coord = this_bb.locspec(this_loc);
            let that_coord = that_bb.locspec(that_loc);
            let ((x1, y1), (x2, y2)) = (this_coord, that_coord);
            let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                this_min_loc = this_loc;
                that_min_loc = that_loc;
            }
        }
    }
    Ok((this_min_loc, that_min_loc))
}

#[derive(PartialEq, Eq)]
struct HeapData {
    cost: u32,
    ind: usize,
}

impl Ord for HeapData {
    // from the exaple heap docs
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.ind.cmp(&other.ind))
    }
}

impl PartialOrd for HeapData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Connector {
    fn loc_to_dir(loc: LocSpec) -> Option<Direction> {
        match loc {
            LocSpec::Top | LocSpec::TopEdge(_) => Some(Direction::Up),
            LocSpec::Right | LocSpec::RightEdge(_) => Some(Direction::Right),
            LocSpec::Bottom | LocSpec::BottomEdge(_) => Some(Direction::Down),
            LocSpec::Left | LocSpec::LeftEdge(_) => Some(Direction::Left),
            _ => None,
        }
    }

    fn parse_element<'a>(
        element: &mut SvgElement,
        elem_map: &'a impl ElementMap,
        start: bool,
    ) -> Result<(
        Option<&'a SvgElement>,
        Option<LocSpec>,
        Option<(f32, f32)>,
        Option<Direction>,
    )> {
        let attrib_name = if start { "start" } else { "end" };
        let this_ref = element
            .pop_attr(attrib_name)
            .ok_or_else(|| SvgdxError::MissingAttribute(attrib_name.to_string()))?;

        let mut t_el: Option<&SvgElement> = None;
        let mut t_loc: Option<LocSpec> = None;
        let mut t_point: Option<(f32, f32)> = None;
        let mut t_dir: Option<Direction> = None;

        // Example: "#thing@tl" => top left coordinate of element id="thing"
        if let Ok((elref, loc)) = parse_el_loc(&this_ref) {
            if let Some(loc) = loc {
                t_dir = Self::loc_to_dir(loc);
                t_loc = Some(loc);
            }
            t_el = elem_map.get_element(&elref);
        } else {
            let mut parts = attr_split(&this_ref).map_while(|v| strp(&v).ok());
            t_point = Some((
                parts.next().ok_or_else(|| {
                    SvgdxError::InvalidData(
                        (attrib_name.to_owned() + "_ref x should be numeric").to_owned(),
                    )
                })?,
                parts.next().ok_or_else(|| {
                    SvgdxError::InvalidData(
                        (attrib_name.to_owned() + "_ref y should be numeric").to_owned(),
                    )
                })?,
            ));
        }

        return Ok((t_el, t_loc, t_point, t_dir));
    }

    fn get_point_along_line(
        el: &SvgElement,
        dist: f32,
    ) -> Result<(f32,f32)>{
        let name = el.name();

        if name == "line"{
            if let (Some(x1),Some(y1),Some(x2),Some(y2)) = (el.get_attr("x1"),el.get_attr("y1"),el.get_attr("x2"), el.get_attr("y2")){
                let x1: f32 = x1.parse()?;
                let y1: f32 = y1.parse()?;
                let x2: f32 = x2.parse()?;
                let y2: f32 = y2.parse()?;
                if x1 == x2 && y1 == y2{
                    return Ok((x1,y1));
                }
                let len = ((x1-x2)*(x1-x2) + (y1-y2)*(y1-y2)).sqrt();
                let rat = dist/len;
                return Ok((x1 + rat*(x2-x1), y1 + rat*(y2-y1)));
            }
        }
        if name == "polyline"{
            if let Some(points) = el.get_attr("points"){
                let points = points.split(", ");
                let mut cummulative_dist = 0.0;
                let mut lastx = 0.0;
                let mut lasty = 0.0;
                let mut first_point = true;
                for p in points{
                    let mut this_point = p.split_whitespace();
                    if let (Some(x),Some(y)) = (this_point.next(),this_point.next()){
                        let x: f32 = x.parse()?;
                        let y: f32 = y.parse()?;

                        if !first_point{
                            let len = ((lastx-x)*(lastx-x) + (lasty-y)*(lasty-y)).sqrt();
                            if cummulative_dist + len > dist{
                                let rat = (dist-cummulative_dist)/len;
                                return Ok((lastx*(1.0-rat) + rat*x, lasty*(1.0-rat) + rat*y));
                            }
                            cummulative_dist += len;
                        }
                        else if dist < 0.0{// clamp to start
                            return Ok((x,y));
                        }
                        lastx = x;
                        lasty = y;
                    }

                    first_point = false;
                }
                return Ok((lastx, lasty));
            }
        }
        if name == "path"{
            if let Some(d) = el.get_attr("d"){

                let replaced_commas = d.replace(&[','], &" ");
                let items = replaced_commas.split_whitespace();

                let mut cummulative_distance = 0.0;
                let mut pos = (0.0,0.0);
                let mut last_stable_pos = pos;
                let mut r = 0.0;
                let mut large_arc_flag = false;
                let mut sweeping_flag = false;

                let mut op = ' ';
                let mut arg_num = 0;
                for item in items{
                    if item.starts_with(&['a','A','c','C','h','H','l','L','m','M','q','Q','s','S','t','T','v','V','z','Z']){
                        if let Some(c) = item.chars().next(){
                            op = c;
                            arg_num = 0;
                            if dist < 0.0 && !['m','M'].contains(&c){// clamping the start
                                return Ok(last_stable_pos);
                            }
                        }
                    }
                    else{
                        if ['c','C','q','Q','s','S','t','T','z','Z'].contains(&op){
                            todo!("not yet impl path parsing");
                        }
                        else if op == 'm'{
                            if arg_num == 0{
                                pos.0 += item.parse::<f32>()?;
                            }
                            else if arg_num == 1{
                                pos.1 += item.parse::<f32>()?;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'M'{
                            if arg_num == 0{
                                pos.0 = item.parse::<f32>()?;
                            }
                            else if arg_num == 1{
                                pos.1 = item.parse::<f32>()?;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'h' || op == 'H'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                if op == 'h'{
                                    pos.0 += val;
                                }
                                else{
                                    pos.0 = val;
                                }
                                let d = (pos.0-last_stable_pos.0).abs();
                                if cummulative_distance + d > dist{
                                    let r = (dist-cummulative_distance)/d;
                                    return Ok((last_stable_pos.0*(1.0-r) + pos.0*r,pos.1));
                                }

                                cummulative_distance += d;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'v' || op == 'V'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                if op == 'v'{
                                    pos.1 += val;
                                }
                                else{
                                    pos.1 = val;
                                }
                                let d = (pos.1-last_stable_pos.1).abs();
                                if cummulative_distance + d > dist{
                                    let r = (dist-cummulative_distance)/d;
                                    return Ok((pos.0, last_stable_pos.1*(1.0-r) + pos.1*r));
                                }

                                cummulative_distance += d;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'l' || op == 'L'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                if op == 'l'{
                                    pos.0 += val;
                                }
                                else{
                                    pos.0 = val;
                                }
                            }
                            else if arg_num == 1{
                                let val = item.parse::<f32>()?;
                                if op == 'l'{
                                    pos.1 += val;
                                }
                                else{
                                    pos.1 = val;
                                }
                                let d = ((last_stable_pos.0-pos.0)*(last_stable_pos.0-pos.0) + (last_stable_pos.1-pos.1)*(last_stable_pos.1-pos.1)).sqrt();
                                if cummulative_distance + d > dist{
                                    let r = (dist-cummulative_distance)/d;
                                    return Ok((last_stable_pos.0*(1.0-r) + pos.0*r,last_stable_pos.1*(1.0-r) + pos.1*r));
                                }

                                cummulative_distance += d;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'a' || op == 'A'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                r = val;
                            }
                            else if arg_num == 1{
                                let val = item.parse::<f32>()?;
                                if r != val{
                                    return Err(SvgdxError::ParseError("path length not supported for non circle elipse".to_string()));
                                }
                            }
                            else if arg_num == 2{
                                // unused as not mean anything for circle
                            }
                            else if arg_num == 3{
                                let val = item.parse::<u32>()?;
                                large_arc_flag = val != 0;
                            }
                            else if arg_num == 4{
                                let val = item.parse::<u32>()?;
                                sweeping_flag = val != 0;
                            }
                            else if arg_num == 5{
                                let val = item.parse::<f32>()?;
                                if op == 'a'{
                                    pos.0 += val;
                                }
                                else{
                                    pos.0 = val;
                                }
                            }
                            else if arg_num == 6{
                                let val = item.parse::<f32>()?;
                                if op == 'a'{
                                    pos.1 += val;
                                }
                                else{
                                    pos.1 = val;
                                }


                                let d2 = (last_stable_pos.0-pos.0)*(last_stable_pos.0-pos.0) + (last_stable_pos.1-pos.1)*(last_stable_pos.1-pos.1);
                                let d = d2.sqrt();
                                
                                let desc = r*r - d2/4.0;
                                let mid_point = ((last_stable_pos.0 + pos.0)*0.5,(last_stable_pos.1 + pos.1)*0.5);
                                let centre;
                                if desc <= 0.0{
                                    centre = mid_point;
                                }
                                else{
                                    let inv_d = 1.0/d;
                                    let perp = ((last_stable_pos.1-pos.1)*inv_d,(pos.0-last_stable_pos.0)*inv_d);
                                    let sign = large_arc_flag ^ sweeping_flag;// which circle to use
                                    let len = if sign {desc.sqrt()} else {-desc.sqrt()};
                                    centre = (mid_point.0 + perp.0*len,mid_point.1 + perp.1*len);
                                }
                                let ang_1 = (last_stable_pos.1-centre.1).atan2(last_stable_pos.0-centre.0);
                                let ang_2 = (pos.1-centre.1).atan2(pos.0-centre.0);
                                
                                let mut shortest_arc_angle = ang_2-ang_1;
                                if shortest_arc_angle < -std::f32::consts::PI{
                                    shortest_arc_angle += std::f32::consts::PI*2.0;
                                }
                                else if shortest_arc_angle > std::f32::consts::PI{
                                    shortest_arc_angle -= std::f32::consts::PI*2.0;
                                }
                                let arc_angle = if large_arc_flag {(std::f32::consts::PI*2.0-shortest_arc_angle.abs())*shortest_arc_angle.signum()} else {shortest_arc_angle};
                                let arc_length = arc_angle.abs()*r;

                                if cummulative_distance + arc_length > dist{
                                    let ratio = (dist-cummulative_distance)/arc_length;
                                    let final_angle = ang_1 + arc_angle*ratio;
                                    
                                    return Ok((centre.0 + r*(final_angle).cos(), centre.1 + r*(final_angle).sin()));
                                }

                                cummulative_distance += arc_length;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        }

                        arg_num += 1;
                    }
                }
                return Ok(pos);
            }
        }

        return Err(SvgdxError::MissingAttribute("in either line, polyline or path".to_string()))
    }

    fn get_line_length(
        el: &SvgElement,
    ) -> Result<f32>{
        let name = el.name();

        let mut sum = 0.0;
        if name == "line"{
            if let (Some(x1),Some(y1),Some(x2),Some(y2)) = (el.get_attr("x1"),el.get_attr("y1"),el.get_attr("x2"), el.get_attr("y2")){
                let x1: f32 = x1.parse()?;
                let y1: f32 = y1.parse()?;
                let x2: f32 = x2.parse()?;
                let y2: f32 = y2.parse()?;
                sum += ((x1-x2)*(x1-x2) + (y1-y2)*(y1-y2)).sqrt();
            }
        }
        if name == "polyline"{
            if let Some(points) = el.get_attr("points"){
                let points = points.split(", ");
                let mut lastx: f32 = 0.0;
                let mut lasty: f32 = 0.0;
                let mut first_point = true;
                for p in points{
                    let mut this_point = p.split_whitespace();
                    if let (Some(x),Some(y)) = (this_point.next(),this_point.next()){
                        let x: f32 = x.parse()?;
                        let y: f32 = y.parse()?;

                        if !first_point{
                            let len: f32 = ((lastx-x)*(lastx-x) + (lasty-y)*(lasty-y)).sqrt();
                            sum += len;
                        }
                        lastx = x;
                        lasty = y;
                    }

                    first_point = false;
                }
            }
        }
        if name == "path"{

            if let Some(d) = el.get_attr("d"){

                let replaced_commas = d.replace(&[','], &" ");
                let items = replaced_commas.split_whitespace();

                let mut cummulative_distance = 0.0;
                let mut pos = (0.0,0.0);
                let mut last_stable_pos = pos;
                let mut r = 0.0;
                let mut large_arc_flag = false;
                let mut sweeping_flag = false;

                let mut op = ' ';
                let mut arg_num = 0;
                for item in items{
                    if item.starts_with(&['a','A','c','C','h','H','l','L','m','M','q','Q','s','S','t','T','v','V','z','Z']){
                        if let Some(c) = item.chars().next(){
                            op = c;
                            arg_num = 0;
                        }
                    }
                    else{
                        if ['c','C','q','Q','s','S','t','T','z','Z'].contains(&op){
                            todo!("not yet impl path parsing");
                        }
                        else if op == 'm'{
                            if arg_num == 0{
                                pos.0 += item.parse::<f32>()?;
                            }
                            else if arg_num == 1{
                                pos.1 += item.parse::<f32>()?;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'M'{
                            if arg_num == 0{
                                pos.0 = item.parse::<f32>()?;
                            }
                            else if arg_num == 1{
                                pos.1 = item.parse::<f32>()?;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'h' || op == 'H'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                if op == 'h'{
                                    pos.0 += val;
                                }
                                else{
                                    pos.0 = val;
                                }
                                let d = (pos.0-last_stable_pos.0).abs();
                                

                                cummulative_distance += d;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'v' || op == 'V'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                if op == 'v'{
                                    pos.1 += val;
                                }
                                else{
                                    pos.1 = val;
                                }
                                let d = (pos.1-last_stable_pos.1).abs();
                                

                                cummulative_distance += d;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'l' || op == 'L'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                if op == 'l'{
                                    pos.0 += val;
                                }
                                else{
                                    pos.0 = val;
                                }
                            }
                            else if arg_num == 1{
                                let val = item.parse::<f32>()?;
                                if op == 'l'{
                                    pos.1 += val;
                                }
                                else{
                                    pos.1 = val;
                                }
                                let d = ((last_stable_pos.0-pos.0)*(last_stable_pos.0-pos.0) + (last_stable_pos.1-pos.1)*(last_stable_pos.1-pos.1)).sqrt();
                                

                                cummulative_distance += d;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        } else if op == 'a' || op == 'A'{
                            if arg_num == 0{
                                let val = item.parse::<f32>()?;
                                r = val;
                            }
                            else if arg_num == 1{
                                let val = item.parse::<f32>()?;
                                if r != val{
                                    return Err(SvgdxError::ParseError("path length not supported for non circle elipse".to_string()));
                                }
                            }
                            else if arg_num == 2{
                                // unused as not mean anything for circle
                            }
                            else if arg_num == 3{
                                let val = item.parse::<u32>()?;
                                large_arc_flag = val != 0;
                            }
                            else if arg_num == 4{
                                let val = item.parse::<u32>()?;
                                sweeping_flag = val != 0;
                            }
                            else if arg_num == 5{
                                let val = item.parse::<f32>()?;
                                if op == 'a'{
                                    pos.0 += val;
                                }
                                else{
                                    pos.0 = val;
                                }
                            }
                            else if arg_num == 6{
                                let val = item.parse::<f32>()?;
                                if op == 'a'{
                                    pos.1 += val;
                                }
                                else{
                                    pos.1 = val;
                                }


                                let d2 = (last_stable_pos.0-pos.0)*(last_stable_pos.0-pos.0) + (last_stable_pos.1-pos.1)*(last_stable_pos.1-pos.1);
                                let d = d2.sqrt();
                                
                                let desc = r*r - d2/4.0;
                                let mid_point = ((last_stable_pos.0 + pos.0)*0.5,(last_stable_pos.1 + pos.1)*0.5);
                                let centre;
                                if desc <= 0.0{
                                    centre = mid_point;
                                }
                                else{
                                    let inv_d = 1.0/d;
                                    let perp = ((last_stable_pos.1-pos.1)*inv_d,(pos.0-last_stable_pos.0)*inv_d);
                                    let sign = large_arc_flag ^ sweeping_flag;// which circle to use
                                    let len = if sign {desc.sqrt()} else {-desc.sqrt()};
                                    centre = (mid_point.0 + perp.0*len,mid_point.1 + perp.1*len);
                                }
                                let ang_1 = (last_stable_pos.1-centre.1).atan2(last_stable_pos.0-centre.0);
                                let ang_2 = (pos.1-centre.1).atan2(pos.0-centre.0);
                                
                                let mut shortest_arc_angle = ang_2-ang_1;
                                if shortest_arc_angle < -std::f32::consts::PI{
                                    shortest_arc_angle += std::f32::consts::PI*2.0;
                                }
                                else if shortest_arc_angle > std::f32::consts::PI{
                                    shortest_arc_angle -= std::f32::consts::PI*2.0;
                                }
                                let arc_angle = if large_arc_flag {(std::f32::consts::PI*2.0-shortest_arc_angle.abs())*shortest_arc_angle.signum()} else {shortest_arc_angle};
                                let arc_length = arc_angle.abs()*r;

                                

                                cummulative_distance += arc_length;
                                last_stable_pos = pos;
                            }
                            else{
                                return Err(SvgdxError::ParseError("path has too many vars".to_string()));
                            }
                        }

                        arg_num += 1;
                    }
                }
                sum += cummulative_distance;
            }
        }

        return Ok(sum);
    }

    fn get_coord_element_loc(
        elem_map: &impl ElementMap,
        el: &SvgElement,
        loc: LocSpec,
    ) -> Result<(f32,f32)>{

        if let LocSpec::PureLength(l) = loc {
            if let Length::Ratio(r) = l{
                let ll = Self::get_line_length(el)?;
                return Self::get_point_along_line(el, ll*r);
            }
            if let Length::Absolute(a) = l{
                return Self::get_point_along_line(el, a);
            }
        }

        let coord = elem_map
            .get_element_bbox(el)?
            .ok_or_else(|| SvgdxError::MissingBoundingBox(el.to_string()))?
            .locspec(loc);

        return Ok(coord);
    }

    pub fn from_element(
        element: &SvgElement,
        elem_map: &impl ElementMap,
        conn_type: ConnectionType,
    ) -> Result<Self> {
        let mut element = element.clone();

        let (start_el, mut start_loc, start_point, mut start_dir) =
            Self::parse_element(&mut element, elem_map, true)?;
        let (end_el, mut end_loc, end_point, mut end_dir) =
            Self::parse_element(&mut element, elem_map, false)?;

        let offset = if let Some(o_inner) = element.pop_attr("corner-offset") {
            Some(
                strp_length(&o_inner)
                    .map_err(|_| SvgdxError::ParseError("Invalid corner-offset".to_owned()))?,
            )
        } else {
            None
        };

        let corner_radius = if let Some(rad) = element.pop_attr("corner-radius") {
            (&rad).parse()
                .map_err(|_| SvgdxError::ParseError("Invalid corner-radius".to_owned()))?
        } else {
            0.0
        };

        // This could probably be tidier, trying to deal with lots of combinations.
        // Needs to support explicit coordinate pairs or element references, and
        // for element references support given locations or not (in which case
        // the location is determined automatically to give the shortest distance)

        let (start, end) = match (start_point, end_point) {
            (Some(start_point), Some(end_point)) => (
                Endpoint::new(start_point, start_dir),
                Endpoint::new(end_point, end_dir),
            ),
            (Some(start_point), None) => {
                let end_el =
                    end_el.ok_or_else(|| SvgdxError::InternalLogicError("no end_el".to_owned()))?;
                if end_loc.is_none() {
                    let eloc = closest_loc(end_el, start_point, conn_type, elem_map)?;
                    end_loc = Some(eloc);
                    end_dir = Self::loc_to_dir(eloc);
                }
                let end_coord = Self::get_coord_element_loc(elem_map, end_el, end_loc.expect("Set from closest_loc"))?;
                (
                    Endpoint::new(start_point, start_dir),
                    Endpoint::new(end_coord, end_dir),
                )
            }
            (None, Some(end_point)) => {
                let start_el = start_el
                    .ok_or_else(|| SvgdxError::InternalLogicError("no start_el".to_owned()))?;
                if start_loc.is_none() {
                    let sloc = closest_loc(start_el, end_point, conn_type, elem_map)?;
                    start_loc = Some(sloc);
                    start_dir = Self::loc_to_dir(sloc);
                }
                let start_coord = Self::get_coord_element_loc(elem_map, start_el, start_loc.expect("Set from closest_loc"))?;
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_point, end_dir),
                )
            }
            (None, None) => {
                let (start_el, end_el) = (
                    start_el
                        .ok_or_else(|| SvgdxError::InternalLogicError("no start_el".to_owned()))?,
                    end_el.ok_or_else(|| SvgdxError::InternalLogicError("no end_el".to_owned()))?,
                );
                if start_loc.is_none() && end_loc.is_none() {
                    let (sloc, eloc) = shortest_link(start_el, end_el, conn_type, elem_map)?;
                    start_loc = Some(sloc);
                    end_loc = Some(eloc);
                    start_dir = Self::loc_to_dir(sloc);
                    end_dir = Self::loc_to_dir(eloc);
                } else if start_loc.is_none() {
                    let end_coord = Self::get_coord_element_loc(elem_map, end_el, end_loc.expect("Not both None"))?;
                    let sloc = closest_loc(start_el, end_coord, conn_type, elem_map)?;
                    start_loc = Some(sloc);
                    start_dir = Self::loc_to_dir(sloc);
                } else if end_loc.is_none() {
                    let start_coord = Self::get_coord_element_loc(elem_map, start_el, start_loc.expect("Not both None"))?;
                    let eloc = closest_loc(end_el, start_coord, conn_type, elem_map)?;
                    end_loc = Some(eloc);
                    end_dir = Self::loc_to_dir(eloc);
                }
                let start_coord = Self::get_coord_element_loc(elem_map, start_el, start_loc.expect("Set above"))?;
                let end_coord = Self::get_coord_element_loc(elem_map, end_el, end_loc.expect("Set above"))?;
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_coord, end_dir),
                )
            }
        };
        Ok(Self {
            source_element: element,
            start,
            end,
            start_el: start_el.cloned(),
            end_el: end_el.cloned(),
            conn_type,
            offset,
            corner_radius,
        })
    }

    fn aals_blocked_by_bb(bb: BoundingBox, a: f32, b: f32, x_axis: bool, axis_val: f32) -> bool {
        if x_axis {
            if axis_val < bb.y1 || axis_val > bb.y2 {
                return false;
            }
            if (a < bb.x1) == (b < bb.x1) && (a > bb.x2) == (b > bb.x2) {
                return false;
            }
        } else {
            if axis_val < bb.x1 || axis_val > bb.x2 {
                return false;
            }
            if (a < bb.y1) == (b < bb.y1) && (a > bb.y2) == (b > bb.y2) {
                return false;
            }
        }

        return true;
    }

    fn render_match_conner(
        &self,
        ratio_offset: f32,
        start_abs_offset: f32,
        end_abs_offset: f32,
        sel_bb: BoundingBox,
        eel_bb: BoundingBox,
        abs_offset_set: bool,
    ) -> Result<Vec<(f32, f32)>> {
        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;

        // method generates all points it could possibly want to go through then does dijkstras on it

        let mut points: Vec<(f32, f32)>;
        if let (Some(start_dir_some), Some(end_dir_some)) = (self.start.dir, self.end.dir) {
            points = vec![];

            // x_lines have constant x vary over y
            let mut x_lines = vec![];
            let mut y_lines = vec![];
            let mut point_set = vec![];
            let mut mid_x = std::usize::MAX;
            let mut mid_y = std::usize::MAX;

            x_lines.push(sel_bb.x1 - start_abs_offset);
            x_lines.push(sel_bb.x2 + start_abs_offset);
            x_lines.push(eel_bb.x1 - end_abs_offset);
            x_lines.push(eel_bb.x2 + end_abs_offset);

            if sel_bb.x1 > eel_bb.x2 {
                // there is a gap
                x_lines.push((sel_bb.x1 + eel_bb.x2) * 0.5);
                mid_x = x_lines.len() - 1;
            } else if sel_bb.x2 < eel_bb.x1 {
                // there is a gap
                x_lines.push((sel_bb.x2 + eel_bb.x1) * 0.5);
                mid_x = x_lines.len() - 1;
            }

            y_lines.push(sel_bb.y1 - start_abs_offset);
            y_lines.push(sel_bb.y2 + start_abs_offset);
            y_lines.push(eel_bb.y1 - end_abs_offset);
            y_lines.push(eel_bb.y2 + end_abs_offset);

            if sel_bb.y1 > eel_bb.y2 {
                // there is a gap
                y_lines.push(sel_bb.y1 * (1.0 - ratio_offset) + eel_bb.y2 * ratio_offset);
                mid_y = y_lines.len() - 1;
            } else if sel_bb.y2 < eel_bb.y1 {
                // there is a gap
                y_lines.push(sel_bb.y2 * (1.0 - ratio_offset) + eel_bb.y1 * ratio_offset);
                mid_y = y_lines.len() - 1;
            }

            match start_dir_some {
                Direction::Left | Direction::Right => {
                    y_lines.push(y1);
                }
                Direction::Down | Direction::Up => {
                    x_lines.push(x1);
                }
            }

            match end_dir_some {
                Direction::Left | Direction::Right => {
                    y_lines.push(y2);
                }
                Direction::Down | Direction::Up => {
                    x_lines.push(x2);
                }
            }

            if abs_offset_set {
                match start_dir_some {
                    Direction::Down => mid_y = 1,  // positive x
                    Direction::Left => mid_x = 0,  // negative y
                    Direction::Right => mid_x = 1, // positive y
                    Direction::Up => mid_y = 0,    // positive x
                }
            }

            for i in 0..x_lines.len() {
                for j in 0..y_lines.len() {
                    point_set.push((x_lines[i], y_lines[j]));
                }
            }

            let mut edge_set = vec![vec![]; point_set.len()];

            for i in 0..point_set.len() {
                for j in 0..point_set.len() {
                    if i == j {
                        continue;
                    }
                    let mut connected = false;

                    // check if not blocked by a wall
                    if point_set[i].0 == point_set[j].0 {
                        if !Self::aals_blocked_by_bb(
                            sel_bb,
                            point_set[i].1,
                            point_set[j].1,
                            false,
                            point_set[i].0,
                        ) && !Self::aals_blocked_by_bb(
                            eel_bb,
                            point_set[i].1,
                            point_set[j].1,
                            false,
                            point_set[i].0,
                        ) {
                            connected = true;
                        }
                    } else if point_set[i].1 == point_set[j].1 {
                        if !Self::aals_blocked_by_bb(
                            sel_bb,
                            point_set[i].0,
                            point_set[j].0,
                            true,
                            point_set[i].1,
                        ) && !Self::aals_blocked_by_bb(
                            eel_bb,
                            point_set[i].0,
                            point_set[j].0,
                            true,
                            point_set[i].1,
                        ) {
                            connected = true;
                        }
                    }
                    if connected {
                        edge_set[i].push(j);
                        edge_set[j].push(i);
                    }
                }
            }

            // just needs to be bigger than 5* (corner cost  +  total bounding box size)
            let inf = 1000000;

            point_set.push((x1, y1)); // start
            point_set.push((x2, y2)); // end
            edge_set.push(vec![]);
            edge_set.push(vec![]);
            let mut queue: BinaryHeap<HeapData> = BinaryHeap::new();
            let mut dist = vec![inf; point_set.len()];

            let start_ind = edge_set.len() - 2;
            let end_ind = edge_set.len() - 1;
            for i in 0..point_set.len() - 2 {
                if (point_set[i].0 == x1 && point_set[i].1 < y1 && start_dir_some == Direction::Up)
                    || (point_set[i].0 == x1
                        && point_set[i].1 > y1
                        && start_dir_some == Direction::Down)
                {
                    if !Self::aals_blocked_by_bb(eel_bb, point_set[i].1, y1, false, x1) {
                        edge_set[i].push(start_ind);
                        edge_set[start_ind].push(i);
                    }
                }
                if (point_set[i].1 == y1
                    && point_set[i].0 > x1
                    && start_dir_some == Direction::Right)
                    || (point_set[i].1 == y1
                        && point_set[i].0 < x1
                        && start_dir_some == Direction::Left)
                {
                    if !Self::aals_blocked_by_bb(eel_bb, point_set[i].0, x1, true, y1) {
                        edge_set[i].push(start_ind);
                        edge_set[start_ind].push(i);
                    }
                }

                if (point_set[i].0 == x2 && point_set[i].1 < y2 && end_dir_some == Direction::Up)
                    || (point_set[i].0 == x2
                        && point_set[i].1 > y2
                        && end_dir_some == Direction::Down)
                {
                    if !Self::aals_blocked_by_bb(sel_bb, point_set[i].1, y2, false, x2) {
                        edge_set[i].push(end_ind);
                        edge_set[end_ind].push(i);
                    }
                }
                if (point_set[i].1 == y2 && point_set[i].0 > x2 && end_dir_some == Direction::Right)
                    || (point_set[i].1 == y2
                        && point_set[i].0 < x2
                        && end_dir_some == Direction::Left)
                {
                    if !Self::aals_blocked_by_bb(sel_bb, point_set[i].0, x2, true, y2) {
                        edge_set[i].push(end_ind);
                        edge_set[end_ind].push(i);
                    }
                }
            }

            // edge cost function
            let corner_cost = 1000;
            let mut edge_costs = vec![vec![]; edge_set.len()];
            for i in 0..edge_set.len() {
                for j in 0..edge_set[i].len() {
                    let ind_1 = i;
                    let ind_2 = edge_set[i][j];

                    let mid_point_mul_x =
                        if mid_x != std::usize::MAX && point_set[ind_1].0 == x_lines[mid_x] {
                            0.5
                        } else {
                            1.0
                        };
                    let mid_point_mul_y =
                        if mid_y != std::usize::MAX && point_set[ind_1].1 == y_lines[mid_y] {
                            0.5
                        } else {
                            1.0
                        };

                    edge_costs[i].push(
                        ((point_set[ind_1].0 - point_set[ind_2].0).abs() * mid_point_mul_y
                            + (point_set[ind_1].1 - point_set[ind_2].1).abs() * mid_point_mul_x)
                            as u32
                            + corner_cost,
                    ); // round may cause some problems
                }
            }

            dist[start_ind] = 0;
            queue.push(HeapData {
                cost: 0,
                ind: start_ind,
            });

            // cant get stuck in a loop as cost for a distance either decreases or queue shrinks
            while !queue.is_empty() {
                let next = queue.pop().expect("would not be in while loop");
                if next.ind == end_ind {
                    break;
                }

                // the node is reached by faster means so already popped
                if next.cost > dist[next.ind] {
                    continue;
                }

                for i in 0..edge_set[next.ind].len() {
                    let edge_cost = edge_costs[next.ind][i];
                    if dist[next.ind] + edge_cost < dist[edge_set[next.ind][i]] {
                        dist[edge_set[next.ind][i]] = dist[next.ind] + edge_cost;
                        queue.push(HeapData {
                            cost: dist[edge_set[next.ind][i]],
                            ind: edge_set[next.ind][i],
                        });
                    }
                }
            }

            let mut back_points_inds = vec![end_ind];
            let mut loc = end_ind;
            while loc != start_ind {
                // would get stuck in a loop if no valid solution
                let mut quit = true;
                for i in 0..edge_set[loc].len() {
                    if dist[edge_set[loc][i]] + edge_costs[loc][i] == dist[loc] {
                        loc = edge_set[loc][i];
                        back_points_inds.push(loc);
                        quit = false;
                        break;
                    }
                }
                if quit {
                    break;
                }
            }

            for i in (0..back_points_inds.len()).rev() {
                points.push(point_set[back_points_inds[i]]);
            }
        } else {
            points = vec![(x1, y1), (x2, y2)];
        }

        return Ok(points);
    }

    pub fn render(&self, ctx: &impl ElementMap) -> Result<SvgElement> {
        let default_ratio_offset = Length::Ratio(0.5);
        let default_abs_offset = Length::Absolute(3.);

        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;
        // For some (e.g. u-shaped) connections we need a default *absolute* offset
        // as ratio (e.g. the overall '50%' default) don't make sense.
        let conn_element = match self.conn_type {
            ConnectionType::Horizontal => {
                // If we have start and end elements, use midpoint of overlapping region
                // TODO: If start_loc is specified, should probably set midpoint
                // to the y coord of that... (implies moving start_loc as an optional
                // inside Connector rather than evaluating it early)
                let midpoint =
                    if let (Some(start_el), Some(end_el)) = (&self.start_el, &self.end_el) {
                        let start_bb = start_el
                            .bbox()?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?;
                        let end_bb = end_el
                            .bbox()?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?;
                        let overlap_top = start_bb
                            .scalarspec(ScalarSpec::Miny)
                            .max(end_bb.scalarspec(ScalarSpec::Miny));
                        let overlap_bottom = start_bb
                            .scalarspec(ScalarSpec::Maxy)
                            .min(end_bb.scalarspec(ScalarSpec::Maxy));
                        (overlap_top + overlap_bottom) / 2.
                    } else {
                        y1
                    };
                SvgElement::new(
                    "line",
                    &[
                        ("x1".to_string(), fstr(x1)),
                        ("y1".to_string(), fstr(midpoint)),
                        ("x2".to_string(), fstr(x2)),
                        ("y2".to_string(), fstr(midpoint)),
                    ],
                )
                .with_attrs_from(&self.source_element)
            }
            ConnectionType::Vertical => {
                // If we have start and end elements, use midpoint of overlapping region
                let midpoint =
                    if let (Some(start_el), Some(end_el)) = (&self.start_el, &self.end_el) {
                        let start_bb = ctx
                            .get_element_bbox(start_el)?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?;
                        let end_bb = ctx
                            .get_element_bbox(end_el)?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?;
                        let overlap_left = start_bb
                            .scalarspec(ScalarSpec::Minx)
                            .max(end_bb.scalarspec(ScalarSpec::Minx));
                        let overlap_right = start_bb
                            .scalarspec(ScalarSpec::Maxx)
                            .min(end_bb.scalarspec(ScalarSpec::Maxx));
                        (overlap_left + overlap_right) / 2.
                    } else {
                        x1
                    };
                SvgElement::new(
                    "line",
                    &[
                        ("x1".to_string(), fstr(midpoint)),
                        ("y1".to_string(), fstr(y1)),
                        ("x2".to_string(), fstr(midpoint)),
                        ("y2".to_string(), fstr(y2)),
                    ],
                )
                .with_attrs_from(&self.source_element)
            }
            ConnectionType::Straight => SvgElement::new(
                "line",
                &[
                    ("x1".to_string(), fstr(x1)),
                    ("y1".to_string(), fstr(y1)),
                    ("x2".to_string(), fstr(x2)),
                    ("y2".to_string(), fstr(y2)),
                ],
            )
            .with_attrs_from(&self.source_element),
            ConnectionType::Corner => {
                let mut abs_offset_set = false;
                let mut start_abs_offset = default_abs_offset.absolute().ok_or("blarg 13872199")?;
                let mut end_abs_offset = start_abs_offset;
                let mut ratio_offset = default_ratio_offset.ratio().ok_or("blarg 13872198")?;
                if let Some(offset) = &self.offset {
                    if let Some(o) = offset.absolute() {
                        start_abs_offset = o;
                        end_abs_offset = o;
                        abs_offset_set = true;
                    }
                    if let Some(r) = offset.ratio() {
                        ratio_offset = r;
                    }
                }

                let mut sel_bb = BoundingBox::new(x1, y1, x1, y1);
                let mut eel_bb = BoundingBox::new(x2, y2, x2, y2);
                if let Some(el) = &self.start_el {
                    if let Ok(Some(el_bb)) = el.bbox() {
                        sel_bb = el_bb;
                    }
                }
                if let Some(el) = &self.end_el {
                    if let Ok(Some(el_bb)) = el.bbox() {
                        eel_bb = el_bb;
                    }
                }
                let points = self.render_match_conner(
                    ratio_offset,
                    start_abs_offset,
                    end_abs_offset,
                    sel_bb,
                    eel_bb,
                    abs_offset_set,
                )?;



                // TODO: remove repeated points.
                if self.corner_radius != 0.0{
                    SvgElement::new(
                        "path",
                        &[(
                            "d".to_string(),
                            Self::points_to_path(points,self.corner_radius)
                        )],
                    )
                    .with_attrs_from(&self.source_element)
                }
                else if points.len() == 2 {
                    SvgElement::new(
                        "line",
                        &[
                            ("x1".to_string(), fstr(points[0].0)),
                            ("y1".to_string(), fstr(points[0].1)),
                            ("x2".to_string(), fstr(points[1].0)),
                            ("y2".to_string(), fstr(points[1].1)),
                        ],
                    )
                    .with_attrs_from(&self.source_element)
                } else {
                    SvgElement::new(
                        "polyline",
                        &[(
                            "points".to_string(),
                            points
                                .into_iter()
                                .map(|(px, py)| format!("{} {}", fstr(px), fstr(py)))
                                .collect::<Vec<String>>()
                                .join(", "),
                        )],
                    )
                    .with_attrs_from(&self.source_element)
                }
            }
        };
        Ok(conn_element)
    }

    fn points_to_path(points: Vec<(f32,f32)>, max_radius: f32) -> String{
        let mut result = String::new();
        let mut radii = vec![];
        for i in 1..(points.len()-1){
            let mut d1 = (points[i].0-points[i-1].0).abs() + (points[i].1-points[i-1].1).abs();
            let mut d2 = (points[i+1].0-points[i].0).abs() + (points[i+1].1-points[i].1).abs();
            if i != 1{
                d1 = d1/2.0;
            }
            if i != points.len()-2{
                d2 = d2/2.0;
            }
            let radius = d1.min(d2).min(max_radius);
            radii.push(radius);
        }

        let mut pos = points[0];
        result += &("M ".to_owned() + &pos.0.to_string() + "," + &pos.1.to_string() + "\n");

        for i in 1..(points.len()-1){
            let dx1 = points[i].0-pos.0;
            let dy1 = points[i].1-pos.1;
            let dx2 = points[i+1].0-points[i].0;
            let dy2 = points[i+1].1-points[i].1;
            
            pos.0 = pos.0 + dx1 - dx1*radii[i-1]/(dx1*dx1+dy1*dy1).sqrt();
            pos.1 = pos.1 + dy1 - dy1*radii[i-1]/(dx1*dx1+dy1*dy1).sqrt();
            
            result += &("L ".to_owned() + &pos.0.to_string() + "," + &pos.1.to_string() + "\n");

            let mut new_pos = points[i];
            
            new_pos.0 = new_pos.0 + dx2*radii[i-1]/(dx2*dx2+dy2*dy2).sqrt();
            new_pos.1 = new_pos.1 + dy2*radii[i-1]/(dx2*dx2+dy2*dy2).sqrt();

            let cl = (dx1*dy2 - dy1*dx2) > 0.0;
            let cl_str = if cl {"1"} else{"0"};

            result += &("a ".to_owned() + &radii[i-1].to_string() + "," + &radii[i-1].to_string() + " 0 0 " + cl_str + " " + &(new_pos.0-pos.0).to_string() + "," + &(new_pos.1-pos.1).to_string() + "\n");

            pos = new_pos;

        }
        pos = points[points.len()-1];
        result += &("L ".to_owned() + &pos.0.to_string() + "," + &pos.1.to_string() + "\n");

        return result;
    }
}
