# Auto-style classes

`svgdx` provides a range of classes which may be added to elements to assign style
and behaviour. By including these classes on elements, associated `<style>` rules and
`<defs>` entries are automatically included.

To avoid conflicts with your own styles, avoid defining `class` or `id` values
beginning `d-`; all classes and id values provided by `svgdx` begin with this prefix.

## Colour - stroke and fill

### `d-<colour>`
Sets the stroke of this element to the given colour, which must be a colour name as
given in the [SVG 'Color' type](https://www.w3.org/TR/SVG11/types.html#DataTypeColor)
or the value `none` to disable stroke.

Note any `text` associated with this element does *not* have its colour changed;
text rendered from `text` attributes has no stroke by default.

Applies to: Basic Shapes

Examples:
```xml
<rect xy="0" wh="10" class="d-red" />
```

### `d-fill-<colour>`
Sets the fill of this element to the given colour, which must be a colour name as given
by the [SVG 'Color' keywords](https://www.w3.org/TR/SVG11/types.html#ColorKeywords) or
the value `none` to disable fill (note `fill: none;` is the default style).

If the fill colour matches an internal list of (subjectively) darker colours,
any `text` associated with this element is changed to render in white rather than black.

Applies to: Basic shapes

Examples:
```xml
<rect xy="0" wh="10" text="Hello!" class="d-fill-deeppink" />
```

## Line styles - dots, dashes, and arrows

### `d-dot` / `d-dash`
Renders an element outline (stroke) with a 'dotted' or 'dashed' line style respectively.
Implemented with `stroke-dasharray`.

### `d-arrow`
Renders an arrowhead at the 'end' of a `line` or `polyline` element. When used on a
connector element (line or polyline with `start` and `end` attributes) the arrowhead
appears at the point pointing toward the `end` point.

## Shadows and gradients

### `d-softshadow` / `d-hardshadow`
Renders a "shadow" filter effect behind the element. `d-softshadow` renders a softer
shadow with a blurred boundary; `d-hardshadow` has more defined boundaries.

Note shadows will extend beyond the bounding-box of an element, and unwanted clipping
of the shadow can be observed in some cases as a result.

TODO: gradients
