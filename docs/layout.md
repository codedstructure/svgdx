# Layout

Most SVG elements are placed on the canvas using absolute coordinates; easy
for SVG-generating tools and GUI applications to handle, but difficult to
manage by hand for anything but the most simple diagrams.

`svgdx` provides various mechanisms to help with laying out diagrams.

> NOTE: `svgdx` assumes that 'User Coordinates' are used for all positioning,
> i.e. [without units](https://www.w3.org/TR/SVG11/coords.html#Units).

> NOTE: Changes to the coordinate system (e.g. using the `transform` attribute)
> are currently ignored when `svgdx` calculates layout.

> NOTE: Bounding boxes are not currently computed for `<path>` elements, so
> these will not position effectively.

## Uniform Attributes

SVG requires different approaches to specifying position and size depending
on the shape being used; `svgdx` makes this uniform by determining axis-aligned
bounding boxes around every element, and placing objects appropriately.

Every object has a width and height (the `wh` attribute), and is located at a
particular point denoting the top-left of the bounding box (the `xy` attribute).
Alternatively, the *center* of an object can be given (via the `cxy` attribute)
along with the width and height.

As usual, mixing and matching with standard SVG attributes is possible, so
an `rx`, `ry` pair may be given alongside an `xy` attribute to define the
position and size of an `<ellipse>` element, for example.

## Relative Positioning

Rather than requiring absolute positions for elements, `svgdx` allows elements
to be placed relative to other elements. Since `xy` defaults to "0" if not
specified, many diagrams will not need any absolute positions to be specified.

The following concepts are defined, and can be combined to make a 'relative
specifier', or 'relspec'.

**Element Reference** - ('elref') may be either 'the previous element' (denoted with
`^`) or an element referenced by its `id`, as `#<id>`, for example `#abc`.

**Location Spec** - ('locspec') a specific point on a given element,
for example 'top-left', or 'center'. These are given by one of the
following abbreviations:

* `tl` - top-left
* `t` - top, i.e. center of the top edge of the bounding box
* `tr` - top-right
* `r` - right, i.e. center of the right-hand edge of the bounding box
* `br` - bottom-right
* `b` - bottom, i.e. center of the bottom edge of the bounding box
* `bl` - bottom-left
* `l` - left, i.e. center of the left-hand edge of the bounding box
* `c` - center of the bounding box

Together an 'elref' and a 'locspec' denote a point in 2D user coordinates.

**Direction Spec** - ('dirspec') denotes a directional relation between
two objects. The following dirspec values are supported:

* `h` - place horizontally to the right of the associated elref
* `H` - place horizontally to the left of the associated elref
* `v` - place vertically below the associated elref
* `V` - place horizontally above the associated elref

### Relspec

The above pieces fit together according to the following grammar.
Note this provides shortcuts rather than being fully consistent,
allowing some separators to be omitted when
context allows it.

```
locspec  := @ [tl|t|tr|r|br|b|bl|l|c]
dirspec  := [h|H|v|V]
prevspec := ^
ident    := alphanumeric
ref      := # ident
elref    := prevspec | ref

relspec  := prevspec dirspec |
            ref : dirspec |
            locspec |
            ref locspec
```

Following the `relspec` as defined above, additional values may be given to define
margins or deltas.

When used as part of a dirspec (e.g. `#abc:H`), a single value defines the 'gap'
between the referenced element and the one being positioned.

When used as part of a locspec (e.g. `@tl`), a pair of values may be provided which
define the `dx` and `dy` offsets to apply.

Some simple examples:

* `xy="#abc:h 5"` - position this element 5 units to the right of the element
  with `id="abc"`.
* `cxy="^"` - position this element to have its center on the center of the
  previous element.
* `xy="^V"` - position this element directly above the previous element.
* `xy="@br"` - position this element at the bottom-right of the
  previous element.
* `xy="#thing@tr 5 10"` - position this element at the top-right of
  element with `id="thing"`, offset by (5, 10).
