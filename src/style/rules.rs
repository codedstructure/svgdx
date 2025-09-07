use std::borrow::Cow;

use crate::types::fstr;

use super::colours::DARK_COLOURS;
use super::types::{MatchResult, MatchType, Rule, Selected, Selector, Style};
use super::{ContextTheme, StyleProvider};

pub(super) struct DefaultStyles {
    seen_svg: bool,
    theme: ContextTheme,
}
impl StyleProvider for DefaultStyles {
    fn new(theme: &ContextTheme) -> Self {
        DefaultStyles {
            theme: theme.clone(),
            seen_svg: false,
        }
    }

    fn get_rules(&self) -> Vec<Rule> {
        let fill = self.theme.theme.default_fill();
        let stroke = self.theme.theme.default_stroke();
        let stroke_width = self.theme.theme.default_stroke_width();
        let font_weight = self.theme.theme.default_font_weight();

        let mut text_styles = vec![
            Style::new("stroke-width", "0"),
            Style::new("font-family", self.theme.font_family.clone()),
            Style::new("font-size", "3px"),
            Style::new("fill", &stroke), // NOTE: text uses stroke as fill
            Style::new("stroke", &fill),
        ];
        if let Some(weight) = font_weight {
            text_styles.push(Style::new("font-weight", weight));
        }

        let mut rules = Vec::with_capacity(16);
        if self.theme.background != "none" {
            rules.push(Rule {
                selector: Selector::Element(MatchType::Element("svg".to_string())),
                styles: vec![Style::new("background", self.theme.background.clone())],
            });
        }

        rules.extend([
            Rule {
                selector: Selector::BasicShape(MatchType::Any),
                styles: vec![
                    Style::new("stroke-width", stroke_width.to_string()),
                    Style::new("fill", &fill),
                    Style::new("stroke", &stroke),
                ],
            },
            Rule {
                selector: Selector::LineLike(MatchType::Any),
                styles: vec![
                    Style::new("stroke-width", stroke_width.to_string()),
                    Style::new("fill", "none"),
                    Style::new("stroke", &stroke),
                ],
            },
            Rule {
                selector: Selector::Element(MatchType::Element("text".to_string())),
                styles: text_styles,
            },
            Rule {
                selector: Selector::Class(MatchType::Class("d-surround".to_string())),
                styles: vec![Style::new("fill", "none")],
            },
        ]);
        rules
    }

    fn on_match(&mut self, _rule: &Rule, selected: &Selected) -> bool {
        // if we've already seen an SVG, veto further matches
        if let Selected::Element(mr) = selected {
            if mr.as_element() == Some("svg".to_string()) {
                if self.seen_svg {
                    return false;
                }
                self.seen_svg = true;
            }
        }
        true
    }
}

pub(super) struct ColourStyles {
    theme: ContextTheme,
}

impl StyleProvider for ColourStyles {
    fn new(theme: &ContextTheme) -> Self {
        ColourStyles {
            theme: theme.clone(),
        }
    }

    fn get_rules(&self) -> Vec<Rule> {
        // Colours
        // - d-colour sets a 'default' colour for shape outlines and text
        // - d-fill-colour sets the colour for shape fills, and sets a text colour
        //   to an appropriate contrast colour.
        // - d-text-colour sets the colour for text elements, which overrides any
        //   colours set by d-colour or d-fill-colour.
        // - d-text-ol-colour sets the colour for text outline
        vec![
            Rule {
                selector: Selector::Class(MatchType::ColourSuffix("d".to_string())),
                styles: vec![Style::new("stroke", "$COLOUR")],
            },
            Rule {
                selector: Selector::TextLike(MatchType::ColourSuffix("d".to_string())),
                styles: vec![
                    Style::new("fill", "$COLOUR"),
                    Style::new("stroke", "$TEXT_ALT_COLOUR"),
                ],
            },
            Rule {
                selector: Selector::Class(MatchType::ColourSuffix("d-fill".to_string())),
                styles: vec![Style::new("fill", "$COLOUR")],
            },
            Rule {
                selector: Selector::TextLike(MatchType::ColourSuffix("d-fill".to_string())),
                styles: vec![
                    Style::new("fill", "$TEXT_ALT_COLOUR"),
                    Style::new("stroke", "$TEXT_OUTLINE_COLOUR"),
                ],
            },
            Rule {
                selector: Selector::TextLike(MatchType::ColourSuffix("d-text".to_string())),
                styles: vec![
                    Style::new("fill", "$COLOUR"),
                    Style::new("stroke", "$TEXT_ALT_COLOUR"),
                ],
            },
            Rule {
                selector: Selector::TextLike(MatchType::ColourSuffix("d-text-ol".to_string())),
                styles: vec![
                    Style::new("stroke", "$COLOUR"),
                    Style::new("stroke-width", "0.5"),
                ],
            },
        ]
    }

    fn on_match(&mut self, _rule: &Rule, selected: &Selected) -> bool {
        if let Selected::TextLike(mr) = selected {
            if mr.as_class() == Some("d-none".to_string()) {
                // Don't set fill:none for text inferred through d-$COLOUR
                return false;
            }
        }
        true
    }

    fn eval_value<'a>(&self, value: &'a str, s: &Selected) -> Cow<'a, str> {
        if let Some(colour) = s.match_result().and_then(|mr| mr.colour()) {
            let is_dark = DARK_COLOURS.binary_search(&colour.as_str()).is_ok();
            let mut value = value.to_string();
            let text_light = self.theme.theme.default_fill();
            let text_dark = self.theme.theme.default_stroke();
            value = value.replace("$COLOUR", &colour.to_string());
            value = value.replace(
                "$TEXT_ALT_COLOUR",
                if is_dark { &text_light } else { &text_dark },
            );
            value = value.replace(
                "$TEXT_OUTLINE_COLOUR",
                if is_dark { &text_dark } else { &text_light },
            );
            Cow::Owned(value)
        } else {
            Cow::Borrowed(value)
        }
    }
}

pub(super) struct TextStyles {
    theme: ContextTheme,
}
impl StyleProvider for TextStyles {
    fn new(theme: &ContextTheme) -> Self {
        TextStyles {
            theme: theme.clone(),
        }
    }

    fn get_rules(&self) -> Vec<Rule> {
        let mut rules = Vec::with_capacity(32);

        // Prevent spiky outlines on text with d-text-ol classes
        rules.push(Rule {
            selector: Selector::Element(MatchType::Element("text".to_string())),
            styles: vec![
                Style::new("stroke-linejoin", "round"),
                Style::new("paint-order", "stroke"),
            ],
        });

        for (class, (key, value)) in [
            ("d-text", ("text-anchor", "middle")),
            ("d-text", ("dominant-baseline", "central")),
            ("d-text-top", ("dominant-baseline", "text-before-edge")),
            ("d-text-bottom", ("dominant-baseline", "text-after-edge")),
            ("d-text-left", ("text-anchor", "start")),
            ("d-text-right", ("text-anchor", "end")),
            ("d-text-top-vertical", ("text-anchor", "start")),
            ("d-text-bottom-vertical", ("text-anchor", "end")),
            (
                "d-text-left-vertical",
                ("dominant-baseline", "text-after-edge"),
            ),
            (
                "d-text-right-vertical",
                ("dominant-baseline", "text-before-edge"),
            ),
        ] {
            rules.push(Rule {
                selector: Selector::TextLike(MatchType::Class(class.to_string())),
                styles: vec![Style::new(key, value)],
            });
        }

        // In SVG1.1, dominant-baseline isn't inherited. SVG2 'corrects' this:
        // https://www.w3.org/TR/SVG11/text.html#DominantBaselineProperty
        // https://www.w3.org/TR/css-inline-3/#propdef-dominant-baseline
        //
        // Firefox / Chrome / Edge all behave with inherited dominant-baseline.
        // For Safari (as of 18.6, 2025-09), it is not inherited, which means
        // either the various text alignment classes need copying to the tspan -
        // bad enough but very noisy in `inline` style mode - or we force it to
        // be inherited, as here. If / when Safari handles dominant-baseline as
        // inherited, this rule can be removed.
        rules.push(Rule {
            selector: Selector::Element(MatchType::Element("tspan".to_string())),
            styles: vec![Style::new("dominant-baseline", "inherit")],
        });

        // The following style rules apply to tspan as well as top-level text elements

        for (class, (key, value)) in [
            // Default is sans-serif 'normal' text.
            ("d-text-bold", ("font-weight", "bold")),
            // Allow explicitly setting 'normal' font-weight, as themes may set a non-normal default.
            ("d-text-normal", ("font-weight", "normal")),
            ("d-text-light", ("font-weight", "100")),
            ("d-text-italic", ("font-style", "italic")),
            ("d-text-monospace", ("font-family", "monospace")),
            ("d-text-pre", ("font-family", "monospace")),
            // experimental!! - should d-text-pre avoid splitting into tspans
            // and NBSP replacement?
            ("d-text-pre", ("white-space", "pre")),
        ] {
            rules.push(Rule {
                selector: Selector::TextLike(MatchType::Class(class.to_string())),
                styles: vec![Style::new(key, value)],
            });
        }

        let text_sizes = [
            ("d-text-smallest", self.theme.font_size * 0.333333),
            ("d-text-smaller", self.theme.font_size * 0.5),
            ("d-text-small", self.theme.font_size * 0.666666),
            ("d-text-medium", self.theme.font_size), // Default, but include explicitly for completeness
            ("d-text-large", self.theme.font_size * 1.5),
            ("d-text-larger", self.theme.font_size * 2.),
            ("d-text-largest", self.theme.font_size * 3.),
        ];
        for (class, size) in text_sizes {
            rules.push(Rule {
                selector: Selector::TextLike(MatchType::Class(class.to_string())),
                styles: vec![Style::new("font-size", format!("{}px", fstr(size)))],
            });
        }
        let text_ol_widths = [
            ("d-text-ol", 0.5), // Must be first, so other classes can override
            ("d-text-ol-thinner", 0.125),
            ("d-text-ol-thin", 0.25),
            ("d-text-ol-medium", 0.5), // Default, but include explicitly for completeness
            ("d-text-ol-thick", 1.),
            ("d-text-ol-thicker", 2.),
        ];
        // TODO: should these be a multiple of the theme stroke-width?
        for (class, width) in text_ol_widths {
            // Selector must be more specific than e.g. `d-thinner`,
            // and must appear after any colour styles, where
            // `d-text-ol-[colour]` provides a default stroke-width.
            rules.push(Rule {
                selector: Selector::TextLike(MatchType::Class(class.to_string())),
                styles: vec![Style::new("stroke-width", fstr(width))],
            });
        }

        rules
    }
}

pub(super) struct StrokeWidthStyles;
impl StyleProvider for StrokeWidthStyles {
    fn new(_theme: &ContextTheme) -> Self {
        StrokeWidthStyles
    }
    fn get_rules(&self) -> Vec<Rule> {
        let mut rules = Vec::with_capacity(16);
        for (class, width) in [
            ("d-thin", 0.25),
            ("d-thinner", 0.125),
            ("d-medium", 0.5), // Default, but include explicitly for completeness
            ("d-thick", 1.),
            ("d-thicker", 2.),
        ] {
            rules.push(Rule {
                selector: Selector::Class(MatchType::Class(class.to_string())),
                styles: vec![Style::new("stroke-width", fstr(width))],
            });
        }
        rules
    }
}

pub(super) struct ArrowStyles {
    need_arrow: bool,
}
impl StyleProvider for ArrowStyles {
    fn new(_theme: &ContextTheme) -> Self {
        ArrowStyles { need_arrow: false }
    }
    fn get_rules(&self) -> Vec<Rule> {
        vec![
            Rule {
                selector: Selector::LineLike(MatchType::Class("d-arrow".to_string())),
                styles: vec![Style::new("marker-end", "url(#d-arrow)")],
            },
            Rule {
                selector: Selector::LineLike(MatchType::Class("d-biarrow".to_string())),
                styles: vec![
                    Style::new("marker-start", "url(#d-arrow)"),
                    Style::new("marker-end", "url(#d-arrow)"),
                ],
            },
        ]
    }

    fn on_match(&mut self, _rule: &Rule, _selected: &Selected) -> bool {
        self.need_arrow = true;
        true
    }

    fn group_styles(&self) -> Vec<String> {
        if self.need_arrow {
            // Safari doesn't support SVG2's 'context-stroke' fill.
            vec!["marker path { fill: inherit; }".to_string()]
        } else {
            Vec::new()
        }
    }

    fn group_defs(&self) -> Vec<String> {
        if self.need_arrow {
            vec![r#"<marker id="d-arrow" refX="1" refY="0.5" orient="auto-start-reverse" markerWidth="6" markerHeight="5" viewBox="0 0 1 1">
  <path d="M 0 0 1 0.4 1 0.6 0 1" style="stroke: none; fill: context-stroke;"/>
</marker>"#.to_string()]
        } else {
            vec![]
        }
    }
}

pub(super) struct DashStyles {
    need_flow: bool,
}

impl StyleProvider for DashStyles {
    fn new(_theme: &ContextTheme) -> Self {
        DashStyles { need_flow: false }
    }

    fn get_rules(&self) -> Vec<Rule> {
        let mut rules = Vec::with_capacity(16);
        for (class, speed) in [
            ("d-flow-slower", "4"),
            ("d-flow-slow", "2"),
            ("d-flow", "1"),
            ("d-flow-fast", "0.5"),
            ("d-flow-faster", "0.25"),
        ] {
            rules.push(Rule {
                selector: Selector::Class(MatchType::Class(class.to_string())),
                styles: vec![
                    Style::new(
                        "animation",
                        format!("{speed}s linear 0s infinite running d-flow-animation"),
                    ),
                    Style::new("stroke-dasharray", "1 1.5"),
                ],
            });
        }
        rules.extend([
            Rule {
                selector: Selector::Class(MatchType::Class("d-flow-rev".to_string())),
                styles: vec![Style::new("animation-direction", "reverse")],
            },
            Rule {
                selector: Selector::Class(MatchType::Class("d-dash".to_string())),
                styles: vec![Style::new("stroke-dasharray", "1 1.5")],
            },
            Rule {
                selector: Selector::Class(MatchType::Class("d-dot".to_string())),
                styles: vec![
                    Style::new("stroke-linecap", "round"),
                    Style::new("stroke-dasharray", "0 1"),
                ],
            },
            Rule {
                selector: Selector::Class(MatchType::Class("d-dot-dash".to_string())),
                styles: vec![
                    Style::new("stroke-linecap", "round"),
                    Style::new("stroke-dasharray", "0 1 1.5 1 0 1.5"),
                ],
            },
        ]);
        rules
    }

    fn group_styles(&self) -> Vec<String> {
        if self.need_flow {
            vec![
                "@keyframes d-flow-animation { from { stroke-dashoffset: 5; } to { stroke-dashoffset: 0; } }".to_string(),
            ]
        } else {
            Vec::new()
        }
    }

    fn on_match(&mut self, _rule: &Rule, _selected: &Selected) -> bool {
        if let Some(class) = _selected.as_class() {
            if class == "d-flow" || class.starts_with("d-flow-") {
                self.need_flow = true;
            }
        }
        true
    }
}

pub(super) struct ShadowStyles {
    need_softshadow: bool,
    need_hardshadow: bool,
}
impl StyleProvider for ShadowStyles {
    fn new(_theme: &ContextTheme) -> Self {
        ShadowStyles {
            need_softshadow: false,
            need_hardshadow: false,
        }
    }
    fn get_rules(&self) -> Vec<Rule> {
        vec![
            Rule {
                selector: Selector::BasicShape(MatchType::Class("d-softshadow".to_string())),
                styles: vec![Style::new("filter", "url(#d-softshadow)")],
            },
            Rule {
                selector: Selector::BasicShape(MatchType::Class("d-hardshadow".to_string())),
                styles: vec![Style::new("filter", "url(#d-hardshadow)")],
            },
        ]
    }

    fn on_match(&mut self, _rule: &Rule, selected: &Selected) -> bool {
        if !self.need_softshadow
            && matches!(selected, Selected::BasicShape(MatchResult::Class(s)) if s == "d-softshadow")
        {
            self.need_softshadow = true;
        } else if !self.need_hardshadow
            && matches!(selected, Selected::BasicShape(MatchResult::Class(s)) if s == "d-hardshadow")
        {
            self.need_hardshadow = true;
        }
        true
    }

    fn group_defs(&self) -> Vec<String> {
        let mut defs = Vec::new();
        if self.need_softshadow {
            defs.push(
                r#"<filter id="d-softshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.7"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.4" k3="1" k4="0"/>
</filter>"#
                    .to_string(),
            );
        }
        if self.need_hardshadow {
            defs.push(
                r#"<filter id="d-hardshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.2"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.6" k3="1" k4="0"/>
</filter>"#
                    .to_string(),
            );
        }
        defs
    }
}

pub(super) struct PatternStyles {
    def_list: Vec<(String, Option<u32>)>,
    theme: ContextTheme,
}

#[derive(Debug, Clone, Copy)]
enum PatternType {
    Horizontal,
    Vertical,
    Grid,
    Stipple,
}

impl PatternStyles {
    const PATTERN_TYPES: &[(&'static str, PatternType, Option<i32>)] = &[
        ("grid", PatternType::Grid, None),
        ("grid-h", PatternType::Horizontal, None),
        ("grid-v", PatternType::Vertical, None),
        ("hatch", PatternType::Horizontal, Some(-45)),
        ("crosshatch", PatternType::Grid, Some(75)),
        ("stipple", PatternType::Stipple, Some(45)),
    ];

    fn pattern_defs(
        &self,
        stroke: &str,
        ptn_id: &str,
        spacing: u32,
        pattern_type: PatternType,
        rotate: Option<i32>,
    ) -> String {
        let rotate = if let Some(r) = rotate {
            format!(" patternTransform=\"rotate({r})\"")
        } else {
            String::new()
        };
        // This is fairly hacky, but a bigger spacing *probably* means
        // covering a larger area and a thicker stroke width is appropriate.
        let sw = fstr((spacing as f32).sqrt() / 10.);
        let mut lines = String::with_capacity(256);
        if let PatternType::Horizontal | PatternType::Grid = pattern_type {
            lines.push_str(&format!(
                r#"<line x1="0" y1="0" x2="{spacing}" y2="0" style="stroke-width: {sw}; stroke: {stroke}"/>"#
            ));
        }
        if let PatternType::Vertical | PatternType::Grid = pattern_type {
            lines.push_str(&format!(
                r#"<line x1="0" y1="0" x2="0" y2="{spacing}" style="stroke-width: {sw}; stroke: {stroke}"/>"#
            ));
        }
        if let PatternType::Stipple = pattern_type {
            let gs = fstr(spacing as f32 / 2.);
            let r = fstr((spacing as f32).sqrt() / 5.);
            lines.push_str(&format!(
                r#"<circle cx="{gs}" cy="{gs}" r="{r}" style="stroke: none; fill: {stroke}"/>"#
            ));
        }
        format!(
            r#"<pattern id="{ptn_id}" x="0" y="0" width="{spacing}" height="{spacing}"{rotate} patternUnits="userSpaceOnUse" >
    <rect width="100%" height="100%" style="stroke: none; fill: none"/>
    {lines}
    </pattern>"#,
        )
    }
}

impl StyleProvider for PatternStyles {
    fn new(theme: &ContextTheme) -> Self {
        PatternStyles {
            def_list: Vec::new(),
            theme: theme.clone(),
        }
    }

    fn get_rules(&self) -> Vec<Rule> {
        let mut rules = Vec::with_capacity(16);
        for (name, _, _) in Self::PATTERN_TYPES {
            let class_name = format!("d-{name}");
            rules.push(Rule {
                selector: Selector::BasicShape(MatchType::Class(class_name.clone())),
                styles: vec![Style::new("fill", format!("url(#{name})"))],
            });
            rules.push(Rule {
                selector: Selector::BasicShape(MatchType::NumericSuffix(class_name.clone())),
                styles: vec![Style::new("fill", format!("url(#{name}-$NUMBER)"))],
            });
        }

        rules
    }

    fn eval_value<'a>(&self, value: &'a str, s: &Selected) -> Cow<'a, str> {
        if let Some(number) = s.match_result().and_then(|mr| mr.number()) {
            let mut value = value.to_string();
            value = value.replace("$NUMBER", &number.to_string());
            Cow::Owned(value)
        } else {
            Cow::Borrowed(value)
        }
    }

    fn on_match(&mut self, _rule: &Rule, selected: &Selected) -> bool {
        if let Some(mr) = selected.match_result() {
            match mr {
                MatchResult::Class(class) => {
                    self.def_list
                        .push((class.trim_start_matches("d-").to_string(), None));
                }
                MatchResult::NumericSuffix(class, num) => {
                    if *num < 1 || *num > 100 {
                        return false; // Invalid numeric suffix for pattern
                    }
                    self.def_list
                        .push((class.trim_start_matches("d-").to_string(), Some(*num)));
                }
                _ => {}
            }
        }
        true
    }

    fn group_defs(&self) -> Vec<String> {
        let stroke = self.theme.theme.default_stroke();
        let mut defs = Vec::new();
        for (pattern_name, spacing) in &self.def_list {
            let (pattern_type, rotate) = if let Some((_, ptype, rot)) = Self::PATTERN_TYPES
                .iter()
                .find(|(p, _, _)| pattern_name == *p)
            {
                (*ptype, *rot)
            } else {
                continue; // Shouldn't happen if rules are derived from PATTERN_TYPES
            };

            let ptn_id = if let Some(spacing) = spacing {
                format!("{pattern_name}-{spacing}")
            } else {
                pattern_name.to_string()
            };
            defs.push(self.pattern_defs(
                &stroke,
                &ptn_id,
                spacing.unwrap_or(1),
                pattern_type,
                rotate,
            ));
        }
        defs
    }
}
