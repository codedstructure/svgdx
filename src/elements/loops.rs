use super::SvgElement;
use crate::context::TransformerContext;
use crate::errors::{Result, SvgdxError};
use crate::events::OutputList;
use crate::expr::{eval_attr, eval_condition, eval_list};
use crate::geometry::{BoundingBox, BoundingBoxBuilder};
use crate::transform::{process_events_with_index, EventGen};

#[derive(Debug, Clone, PartialEq)]
enum LoopType {
    Repeat(String),
    While(String),
    Until(String),
}

#[derive(Debug, Clone, PartialEq)]
struct LoopDef {
    loop_type: LoopType,
    loop_spec: Option<(String, String, String)>,
}

impl TryFrom<&SvgElement> for LoopDef {
    type Error = SvgdxError;

    fn try_from(element: &SvgElement) -> Result<Self> {
        if element.name() != "loop" {
            return Err(SvgdxError::InvalidData(
                "LoopType can only be created from a loop element".to_string(),
            ));
        }
        let loop_spec = if let Some(loop_var) = element.get_attr("loop-var") {
            // Note we don't parse attributes here as they might be expressions,
            // and we don't have access to a context to evaluate them
            let start = element.get_attr("start").unwrap_or("0");
            let step = element.get_attr("step").unwrap_or("1");
            Some((loop_var.to_string(), start.to_string(), step.to_string()))
        } else {
            None
        };
        let loop_type;
        if let Some(count) = element.get_attr("count") {
            loop_type = LoopType::Repeat(count.to_string());
        } else if let Some(while_expr) = element.get_attr("while") {
            loop_type = LoopType::While(while_expr.to_string());
        } else if let Some(until_expr) = element.get_attr("until") {
            loop_type = LoopType::Until(until_expr.to_string());
        } else {
            return Err(SvgdxError::MissingAttribute(
                "count | while | until".to_string(),
            ));
        }
        Ok(Self {
            loop_type,
            loop_spec,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LoopElement(pub SvgElement);

impl EventGen for LoopElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let event_element = &self.0;
        let mut gen_events = OutputList::new();
        let mut bbox = BoundingBoxBuilder::new();
        if let (Ok(loop_def), Some(inner_events)) = (
            LoopDef::try_from(event_element),
            event_element.inner_events(context),
        ) {
            let mut iteration = 0;
            let mut loop_var_name = String::new();
            let mut loop_count = 0;
            let mut loop_var_value = 0.;
            let mut loop_step = 1.;
            if let LoopType::Repeat(count) = &loop_def.loop_type {
                loop_count = eval_attr(count, context)?.parse()?;
            }
            if let Some((loop_var, start, step)) = loop_def.loop_spec {
                loop_var_name = eval_attr(&loop_var, context)?;
                loop_var_value = eval_attr(&start, context)?.parse()?;
                loop_step = eval_attr(&step, context)?.parse()?;
            }

            loop {
                if let LoopType::Repeat(_) = &loop_def.loop_type {
                    if iteration >= loop_count {
                        break;
                    }
                } else if let LoopType::While(expr) = &loop_def.loop_type {
                    if !eval_condition(expr, context)? {
                        break;
                    }
                }

                if !loop_var_name.is_empty() {
                    context.set_var(&loop_var_name, &loop_var_value.to_string());
                }

                // Each iteration needs different order indices on elements, so e.g.
                // ElRef::Prev isn't identical for each iteration.
                let iter_oi = event_element.order_index.with_index(iteration as usize);
                let (ev_list, ev_bbox) =
                    process_events_with_index(inner_events.clone(), context, Some(iter_oi))?;
                gen_events.extend(&ev_list);
                if let Some(bb) = ev_bbox {
                    bbox.extend(bb);
                }

                if let LoopType::Until(expr) = &loop_def.loop_type {
                    if eval_condition(expr, context)? {
                        break;
                    }
                }
                iteration += 1;
                loop_var_value += loop_step;
                if iteration > context.config.loop_limit {
                    return Err(SvgdxError::LoopLimitError(
                        iteration,
                        context.config.loop_limit,
                    ));
                }
            }
        }
        Ok((gen_events, bbox.build()))
    }
}

struct ForDef {
    var_name: String,
    idx_name: Option<String>,
    data: String,
}

impl TryFrom<&SvgElement> for ForDef {
    type Error = SvgdxError;

    fn try_from(element: &SvgElement) -> Result<Self> {
        let var_name = element
            .get_attr("var")
            .ok_or_else(|| SvgdxError::MissingAttribute("var".to_string()))?;
        let idx_name = element.get_attr("idx-var");
        let data = element
            .get_attr("data")
            .ok_or_else(|| SvgdxError::MissingAttribute("data".to_string()))?;
        Ok(Self {
            var_name: var_name.to_string(),
            idx_name: idx_name.map(|s| s.to_string()),
            data: data.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ForElement(pub SvgElement);

impl EventGen for ForElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let event_element = &self.0;
        let mut gen_events = OutputList::new();
        let mut bbox = BoundingBoxBuilder::new();
        let mut idx = 0;
        if let (Ok(for_def), Some(inner_events)) = (
            ForDef::try_from(event_element),
            event_element.inner_events(context),
        ) {
            let data_list: Vec<_> = eval_list(&for_def.data, context)?;
            let idx_name = for_def.idx_name.clone();

            // TODO: should a new context be created for for loops, so
            // loop & idx vars don't leak out / override existing vars?
            for item in data_list {
                context.set_var(&for_def.var_name, &item);
                if let Some(ref idx_name) = idx_name {
                    context.set_var(idx_name, &idx.to_string());
                }
                let iter_oi = event_element.order_index.with_index(idx as usize);
                let (ev_list, ev_bbox) =
                    process_events_with_index(inner_events.clone(), context, Some(iter_oi))?;
                gen_events.extend(&ev_list);
                if let Some(bb) = ev_bbox {
                    bbox.extend(bb);
                }
                idx += 1;
                if idx > context.config.loop_limit {
                    return Err(SvgdxError::LoopLimitError(idx, context.config.loop_limit));
                }
            }
            Ok((gen_events, bbox.build()))
        } else {
            Err(SvgdxError::InvalidData("Invalid <for> element".to_string()))
        }
    }
}
