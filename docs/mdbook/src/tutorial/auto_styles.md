# Delta 3 - Auto-styles

> svgdx provides a range of CSS classes which control style and behaviour

## Auto-style classes

Built-in svgdx class names all have a `d-` prefix.  To avoid conflicts with your own styles, avoid defining `class` or `id` values beginning `d-`.

Note that inclusion of the relevant CSS definitions is automatic based on which classes are defined on properties; while this makes changing classes in the generated SVG document less convenient, but avoids including large chunks of mostly-unused style definitions in the output.

Adding these auto-style classes can be used to both affect presentation style and control layout and positioning.

## Presentation style

### Colour - stroke and fill

Three class definition formats affect colour:

- [`d-<colour>`](#d-colour) sets a 'default' colour for shape outlines and text
- [`d-fill-<colour>`](#d-fill-colour) sets the colour for shape fills, and sets a default text colour to an appropriate contrast colour, if not overridden by `d-fill-<colour>` or `d-text-<colour>`
- [`d-text-<colour>`](#d-text-colour) sets the colour for text elements, which overrides any implicit text colour set by `d-colour` or `d-fill-colour`.

> Note that the approach to colour in auto-styles assumes that text will not have a `stroke` outline; if text stroke needs to be specified, use custom classes and styles.

#### `d-<colour>`
Sets the stroke of this element to the given colour, which must be a colour name as given in the [SVG 'Color' type](https://www.w3.org/TR/SVG11/types.html#DataTypeColor) or the value `none` to disable stroke.

By default any `text` associated with this element will also have its colour changed, though text colour is applied via the `fill` attribute rather than `stroke`. Text colour can be overridden by use of [`d-text-<colour>`](#d-text-colour).

> An exception is that `d-none` (typically used to prevent an outline being rendered) does not apply an equivalent to text, since this would leave the text invisible.

Applies to: Basic Shapes

Examples:
```xml
<rect xy="0" wh="10" class="d-red" />
```

#### `d-fill-<colour>`
Sets the fill of this element to the given colour, which must be a colour name as given
by the [SVG 'Color' keywords](https://www.w3.org/TR/SVG11/types.html#ColorKeywords) or
the value `none` to disable fill (note `fill: none;` is the default style).

If the fill colour matches an internal list of (subjectively) darker colours,
any `text` associated with this element is changed to render in white rather than black, unless overridden by [`d-<colour>`](#d-colour) or [`d-text-colour`](#d-text-colour).

Applies to: Basic shapes

Examples:
```xml
<rect xy="0" wh="10" text="Hello!" class="d-fill-deeppink" />
```

#### `d-text-<colour>`
Sets the colour of rendered text to the given colour, which must be a colour name as given
by the [SVG 'Color' keywords](https://www.w3.org/TR/SVG11/types.html#ColorKeywords).
Note this overrides any colour applied by the other colour specifiers above.

Example:
```xml
<rect xy="0" wh="10" text="Hello!" class="d-fill-grey d-text-darkblue d-green" />
```

This will render a grey square with green outline and dark blue text.

### Text styles

#### `d-text-smallest` / `-smaller` / `-small` / `-medium` / `-large` / `-larger` / `-largest`

These styles control the size of text. The default text size is `d-text-medium`, but providing
this style as an option allows the various relative size styles to be used if global font-size
is overriden.

### `d-text-monospace` / `d-text-italic` / `d-text-bold`

These styles provide basic styling of text elements, and may be combined as required.

### `d-text-pre`

This style is similar to `d-text-monospace`, but in addition the text element has spaces
replaced with non-breaking spaces. This prevents the usual XML whitespace collapse which
replaces multiple contiguous spaces with a single space.

> The NBSP replacement approach may change in future,
> as SVG2 has better support for preserving whitespace.

This is useful for including code listings, ASCII art, and other whitespace-sensitive text
in an SVG document.

## Line styles - dots, dashes, and arrows

### `d-dot` / `d-dash`
Renders an element outline (stroke) with a 'dotted' or 'dashed' line style respectively.
Implemented with `stroke-dasharray`.

### `d-thin` / `d-thick`
These respectively reduce or increase the stroke width from the default by a factor of 2.

### `d-arrow`
Renders an arrowhead at the 'end' of a `line` or `polyline` element. When used on a
connector element (line or polyline with `start` and `end` attributes) the arrowhead
appears at the point pointing toward the `end` point.

### `d-flow`
Animates (using CSS) the `stroke-dashoffset` property, to provide the appearance of
flowing lines. The simple `d-flow` property adds the equivalent of `d-dash` by default,
but providing the `d-dot` property will override this.

Different speeds can be provided by using the suffixes `slower`, `slow`, `fast` or
`faster`, and the direction can be reversed by providing the *additional* class
`d-flow-rev`. For a 'dotted fast reverse flow', use `class="d-dot d-flow-fast d-flow-rev"`.

This style provides interesting effects beyond lines - try on circles with radius a
multiple of pi.

## Shadows and gradients

### `d-softshadow` / `d-hardshadow`
Renders a "shadow" filter effect behind the element. `d-softshadow` renders a softer
shadow with a blurred boundary; `d-hardshadow` has more defined boundaries.

Note shadows will extend beyond the bounding-box of an element, and unwanted clipping
of the shadow can be observed in some cases as a result.

## Patterns

### `d-grid` / `d-grid-N`
These classes define a fill for the associated object which draw thin grid lines at
gaps of 1, or N (1-100) respectively. This can be useful when debugging a diagram.

### `d-stipple` / `d-hatch` / `d-crosshatch`
These classes provide various fill patterns.


TODO: gradients
