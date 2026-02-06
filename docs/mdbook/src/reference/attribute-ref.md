# Attributes

## General attributes

### `id`
This has the same meaning as in normal SVG (and XML); it should be unique within the document, and will be transferred as-is to the output.

### `class`
This has the same meaning as in normal SVG, but a set of built-in [auto-styles](auto_styles.md) may have side-effects which affect conversion to SVG.

### `_`, `__`
These attributes are used to attach a comment to an input element, which will be converted into an XML Comment prior to the generated element(s).

Expressions and variables in the `_` attribute will be evaluated, while `__` is a 'raw' comment, with no special processing.

Example
```xml
<rect id="base" wh="10" _="All other elements are positioned relative to this"/>
```

## Position and size

### `xy`
Determines the top-left point of the given shape.

Note that the top-left point is calculated via the bounding box, and may not actually be part of the shape itself, e.g. in the case of a `<circle>`.

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: Basic shapes

Example
```xml
<rect xy="10 5" width="5" height="5" />
```

### `cxy`
Determines the center point of the given shape.

Note that the center point is calculated via a bounding box on the shape, unless the SVG shape itself natively supports `cx`, `cy` (i.e. `<circle>`, `<ellipse>`)

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: Basic shapes

### `xy-loc`
Overrides the behaviour of `xy` to indicate another point on the bounding box of the given shape.

Ignored if `xy` is not given.

Type: [Location](#location)

For example, `xy-loc="c"` (using the `c` or 'center' [location](#location)) makes an `xy` attribute
behave the same as if just `cxy` was provided.

May be used for relative alignment, e.g. in the following the second rectangle is positioned with it's left (`l`) point equal to the right (`r`) location of the first rectangle.
```xml
<rect id="a" xy="0" wh="10" />
<rect xy="#a@r" xy-loc="l" wh="10" />
```

Applies to: Basic shapes

### `dx`, `dy`, `dxy`

TODO

### `wh`
Determines the width and height of the given shape.

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: `<rect>`, `<circle>`, `<ellipse>`

### `dw`, `dh`, `dwh`

TODO

### `surround`, `inside`
As an alternative to specifying position and size (e.g. `xy` and `wh`),
the `surround` or `inside` attributes can be given a list of element
references, causing it to be positioned at either the union (`surround`)
or intersection (`inside`) of the bounding boxes to those elements.
May be used together with the [`margin`](#margin) attribute to visually
group a set of related elements.

Type: [List](#lists) of [Element ref](#element-ref) items.

Applies to: `<rect>`, `<circle>`, `<ellipse>`

Example:
```xml
<rect id="a" xy="0" wh="2" />
<rect id="b" xy="5 0" wh="2" />
<rect id="c" xy="0 5" wh="2" />
<rect surround="#a #b" margin="1" class="d-dash" />
```

### `margin`
**Note:** The behaviour of `margin` is context-dependent and has no
meaning in isolation.

Typically it represents added space / size (see also [dw / dh / dwh](#dw-dh-dwh))
between or around elements.

When used with `inside`, the margin is a *decrease* in the target element size
relative to the intersection box; when used with `surround`, margin is an
*increase* in the target element size.

Separate margins may be defined for each of the 'TRBL' (top, right, bottom, left)
edges analogous to [CSS `padding` and `margin` values](https://developer.mozilla.org/en-US/docs/Web/CSS/Shorthand_properties#margin_and_padding_properties).

* If a **single value** is given it is used for all 4 edges.
* If **2 values** are given they correspond to top/bottom and left/right edges respectively.
* If **3 values** are given, they correspond to top, left/right, bottom edges respectively.
* If **4 values** are given, they correspond to top, right, bottom, left edges
  (i.e. clockwise from top) respectively.

Each entry in a `margin` attribute may be either a number (in user coordinates)
or a percentage length.

## Lines and connectors

### `xy1`
Determines the starting point of a `<line>` element.

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: `<line>` elements.

Example:
```xml
<line xy1="0" xy2="10 20" />
```

### `xy2`
Determines the ending point of a `<line>` element.

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: `<line>` elements.

Example:
```xml
<line xy1="0" xy2="10 20" />
```

### `start`
Determines the ending point of a connector.
This may be a simple [expression pair](#expression-pair), (in which case it acts identically to `xy1`) but is typically relative to another shape element.

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: `<line>`, `<polyline>` elements.

Example:
```xml
<line start="#abc" end="#pqr" />
```

### `end`
Determines the ending point of a connector.
This may be a simple [expression pair](#expression-pair), (in which case it acts identically to `xy2`) but is typically relative to another shape element.

Type: [Expression pair](#expression-pair); [Relative specifier](layout#relative-positioning)

Applies to: `<line>`, `<polyline>` elements.

Example:
```xml
<line start="#abc" end="#pqr" />
```

### `corner-offset`

TODO

## Text attributes

### `text`
Provides a text string to associate with and display on the given element.

TODO: expand

### `text-loc`
Determines the location of element text. Behaviour depends on the element this applies to:
For shapes enclosing an area (i.e. not simple lines) the text is assumed to live 'inside' the shape,
and the location determines 'text justification' in both horizontal and vertical aspects.

Type: [Location](#location)

Applies to: Basic shapes

### `text-offset`
When `text-loc` is used to place text at the corner or edge of a shape, it can become unreadable if pushed all the way to the edge.
The `text-offset` attribute - which defaults to '1' if omitted - controls how much the text string is 'pulled in' from the edge or corner.

For centered text this has no effect.

# Types

## Lists
List attributes correspond to SVG 1.1's `<list-of-Ts>` datatype:
> "A list consists of a separated sequence of values. Unless explicitly described differently, lists within SVG's XML attributes can be either comma-separated, with optional white space before or after the comma, or white space-separated."

[source](https://www.w3.org/TR/SVG11/types.html#DataTypeList)

## Expression pair

## Location
