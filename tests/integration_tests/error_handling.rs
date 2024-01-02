// Error handling tests.

// Note that transform() is quite lenient, as trailing text can cover
// things which would otherwise be expected to be problems, e.g.
// `<!--rect x="-->"/>` will be treated as a comment followed by the
// Text type containing `"/>`.

use svgdx::transform_str;

#[test]
fn test_error_bad_tag() {
    let input = r##"<svg>
    <rect>
    </svg>"##;

    assert!(transform_str(input).is_err());
}

#[test]
fn test_error_bad_element() {
    let input = r##"<svg>
    <rect
    </svg>"##;

    assert!(transform_str(input).is_err());
}

#[test]
fn test_error_bad_comment() {
    let input = r##"<svg>
    <!--rect>
    </svg>"##;

    assert!(transform_str(input).is_err());
}

#[test]
fn test_error_attr() {
    let input = r##"<svg>
    <rect x=y/>
    </svg>"##;
    assert!(transform_str(input).is_err());

    let input = r##"<svg>
    <rect x="y/>
    </svg>"##;
    assert!(transform_str(input).is_err());

    let input = r##"<svg>
    <rect x='y"/>
    </svg>"##;
    assert!(transform_str(input).is_err());

    let input = r##"<svg>
    <rect x="y" a/>
    </svg>"##;
    assert!(transform_str(input).is_err());
}

#[test]
fn test_error_bad_attr_value() {
    let input = r##"<svg>
    <rect xy="#a"/>
    </svg>"##;
    assert!(transform_str(input).is_err());

    let input = r##"<svg>
    <rect xy="0" dx="abc"/>
    </svg>"##;
    assert!(transform_str(input).is_err());

    let input = r##"<svg>
    <rect xy="0" dx="0.a"/>
    </svg>"##;
    assert!(transform_str(input).is_err());
}
