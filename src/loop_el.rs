use crate::context::TransformerContext;
use crate::element::SvgElement;
use crate::events::EventList;
use crate::expression::{eval_attr, eval_condition};
use crate::position::{BoundingBox, BoundingBoxBuilder};
use crate::transform::{process_events, ElementLike};

use anyhow::{bail, Result};

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
    type Error = anyhow::Error;

    fn try_from(element: &SvgElement) -> Result<Self> {
        if element.name != "loop" {
            bail!("LoopType can only be created from a loop element");
        }
        let loop_spec = if let Some(loop_var) = element.get_attr("loop-var") {
            // Note we don't parse attributes here as they might be expressions,
            // and we don't have access to a context to evaluate them
            let start = element.get_attr("start").unwrap_or("0".to_string());
            let step = element.get_attr("step").unwrap_or("1".to_string());
            Some((loop_var, start, step))
        } else {
            None
        };
        let loop_type;
        if let Some(count) = element.get_attr("count") {
            loop_type = LoopType::Repeat(count); //, loop_spec));
        } else if let Some(while_expr) = element.get_attr("while") {
            loop_type = LoopType::While(while_expr);
        } else if let Some(until_expr) = element.get_attr("until") {
            loop_type = LoopType::Until(until_expr);
        } else {
            bail!("Loop element should have a count, while or until attribute");
        }
        Ok(Self {
            loop_type,
            loop_spec,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LoopElement(pub SvgElement); // LoopDef);

impl ElementLike for LoopElement {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(EventList, Option<BoundingBox>)> {
        let event_element = &self.0;
        let mut gen_events = EventList::new();
        let mut bbox = BoundingBoxBuilder::new();
        if let (Ok(loop_def), Some((start, end))) =
            (LoopDef::try_from(event_element), event_element.event_range)
        {
            // opening loop element is not included in the processed inner events to avoid
            // infinite recursion...
            let inner_events = EventList::from(context.events.clone()).slice(start + 1, end);

            let mut iteration = 0;
            let mut loop_var_name = String::new();
            let mut loop_count = 0;
            let mut loop_var_value = 0.;
            let mut loop_step = 1.;
            if let LoopType::Repeat(count) = &loop_def.loop_type {
                loop_count = eval_attr(count, context).parse()?;
            }
            if let Some((loop_var, start, step)) = loop_def.loop_spec {
                loop_var_name = eval_attr(&loop_var, context);
                loop_var_value = eval_attr(&start, context).parse()?;
                loop_step = eval_attr(&step, context).parse()?;
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

                let (ev_list, ev_bbox) = process_events(inner_events.clone(), context)?;
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
                    bail!("Excessive looping detected");
                }
            }
        }
        Ok((gen_events, bbox.build()))
    }

    fn get_element(&self) -> Option<SvgElement> {
        Some(self.0.clone())
    }

    fn get_element_mut(&mut self) -> Option<&mut SvgElement> {
        Some(&mut self.0)
    }
}
