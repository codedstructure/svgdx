# Attributes

## Position and size

### `xy`
Determines the top-left point of the given shape.

Note that the top-left point is calculated via the bounding box, and may not actually be part of the shape itself, e.g. in the case of a `<circle>`.

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

Applies to: Basic shapes

Example
```xml
<rect xy="10 5" width="5" height="5" />
```

### `cxy`
Determines the center point of the given shape.

Note that the center point is calculated via a bounding box on the shape, unless the SVG shape itself natively supports `cx`, `cy` (i.e. `<circle>`, `<ellipse>`)

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

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

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

Applies to: `<rect>`, `<circle>`, `<ellipse>`

### `dw`, `dh`, `dwh`

TODO

## Lines and connectors

### `xy1`
Determines the starting point of a `<line>` element.

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

Applies to: `<line>` elements.

Example:
```xml
<line xy1="0" xy2="10 20" />
```

### `xy2`
Determines the ending point of a `<line>` element.

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

Applies to: `<line>` elements.

Example:
```xml
<line xy1="0" xy2="10 20" />
```

### `start`
Determines the ending point of a connector.
This may be a simple [expression pair](#expression-pair), (in which case it acts identically to `xy1`) but is typically relative to another shape element.

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

Applies to: `<line>`, `<polyline>` elements.

Example:
```xml
<line start="#abc" end="#pqr" />
```

### `end`
Determines the ending point of a connector.
This may be a simple [expression pair](#expression-pair), (in which case it acts identically to `xy2`) but is typically relative to another shape element.

Type: [Expression pair](#expression-pair); [Relative specifier](#relative-specifier)

Applies to: `<line>`, `<polyline>` elements.

Example:
```xml
<line start="#abc" end="#pqr" />
```

### `edge-type`

TODO

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

# Types

## Expression pair

## Location

## Relative specifier
