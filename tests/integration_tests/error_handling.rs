// Error handling tests.

// Note that transform() is quite lenient, as trailing text can cover
// things which would otherwise be expected to be problems, e.g.
// `<!--rect x="-->"/>` will be treated as a comment followed by the
// Text type containing `"/>`.

use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_error_bad_tag() {
    let input = r##"<svg>
    <rect>
    </svg>"##;

    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_error_bad_element() {
    let input = r##"<svg>
    <rect
    </svg>"##;

    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_error_bad_comment() {
    let input = r##"<svg>
    <!--rect>
    </svg>"##;

    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_error_attr() {
    let input = r##"<svg>
    <rect x=y/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());

    let input = r##"<svg>
    <rect x="y/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());

    let input = r##"<svg>
    <rect x='y"/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());

    let input = r##"<svg>
    <rect x="y" a/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_error_bad_attr_value() {
    let input = r##"<svg>
    <rect xy="#a"/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());

    let input = r##"<svg>
    <rect xy="0" dx="abc"/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());

    let input = r##"<svg>
    <rect xy="0" dx="0.a"/>
    </svg>"##;
    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_error_mode() {
    let input = |mode, value| {
        format!(
            r##"<svg>
<config error-mode="{mode}"/>
<rect wh="{value}"/>
</svg>"##
        )
    };

    for (mode, value, ok) in &[
        ("strict", "abc", false),
        ("warn", "abc", true),
        ("ignore", "abc", true),
        ("strict", "1", true),
        ("warn", "1", true),
        ("ignore", "1", true),
    ] {
        assert!(transform_str_default(input(mode, value)).is_ok() == *ok);
    }
}

#[test]
fn test_error_mode_warn() {
    let input = |mode| {
        format!(
            r##"<svg>
<config error-mode="{mode}" auto-style-mode="none"/>
<rect x="abc"/>
</svg>"##
        )
    };

    for mode in ["warn", "ignore"] {
        let output = transform_str_default(input(mode)).unwrap();
        assert_contains!(output, r#"<rect x="abc"/>"#);
    }

    let ignore_output = transform_str_default(input("ignore")).unwrap();
    assert_eq!(
        ignore_output,
        r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg">
<rect x="abc"/>
</svg>"##
    );
}
