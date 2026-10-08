mod attr_expansion;
mod auto_id;
mod auto_styles;
mod box_element;
mod clippath;
#[cfg(feature = "cli")]
mod cmdline;
mod comments;
mod config;
mod connector;
mod containment;
mod defaults;
mod error_handling;
mod eval_locs;
mod expression;
mod gradient;
mod group;
mod if_element;
mod indent;
mod loops;
mod point;
mod position;
mod rel_pos;
mod rel_size;
mod rel_xfrm;
mod reuse;
mod root_svg;
mod rotate;
mod roundtrip;
mod src_line;
mod text_attr;
mod transform_svg;
mod variables;

macro_rules! assert_contains {
    ($container:expr, $content:expr) => {
        if !$container.contains($content) {
            panic!(
                "Failed assert_contains:\n\nContainer:\n```\n{}\n```\nContent:\n```\n{}\n```\n",
                $container, $content
            );
        }
    };
}

macro_rules! assert_not_contains {
    ($container:expr, $content:expr) => {
        if $container.contains($content) {
            panic!(
                "Failed assert_not_contains:\n\nContainer:\n```\n{}\n```\nContent:\n```\n{}\n```\n",
                $container, $content
            );
        }
    };
}

pub(crate) use {assert_contains, assert_not_contains};
