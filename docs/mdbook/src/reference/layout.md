# Layout

Most SVG elements are placed on the canvas using absolute coordinates; easy
for SVG-generating tools and GUI applications to handle, but difficult to
manage by hand for anything but the most simple diagrams.

`svgdx` provides various mechanisms to help with laying out diagrams.

> NOTE: `svgdx` assumes that 'User Coordinates' are used for all positioning,
> i.e. [without units](https://www.w3.org/TR/SVG11/coords.html#Units).

> NOTE: Changes to the coordinate system (e.g. using the `transform` attribute)
> are currently ignored when `svgdx` calculates layout.

> NOTE: Bounding boxes calculations for `<path>` elements are incomplete,
> (in particular arcs and curves are not handled) so these may not position
> effectively.

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
specified, many diagrams will not need _any_ absolute positions to be specified.

The following concepts are defined, and can be combined to make a 'relative
specifier', or 'relspec'.

**Layout Reference** - ('layoutref') denotes something with a bounding box.
This is typically another element, but may also be derived from several
elements. The following forms are supported:

* An element reference ('elref') - either 'the previous element' (denoted with
  `^`) or an element referenced by its `id`, as `#<id>`, for example `#abc`.
* A union of element references, given as a space-separated list of elrefs
  inside square brackets, for example `[#abc #def ^]`. The resulting bbox is
  the union of the bounding boxes of every listed element.

A union has a bounding box but no inherent shape, so any location or scalar
value is derived from the bounding box. The elements in a union may themselves
be positioned relative to other layout references, but unions cannot be nested
directly (e.g. `[[#a #b] #c]` is not valid).

**Location Spec** - ('locspec') a specific point on a given layoutref,
for example 'top-left', 'center', or '75% along the top edge'.
These are given by one of the following abbreviations:

* `tl` - top-left
* `t` - top, i.e. center of the top edge of the bounding box
* `tr` - top-right
* `r` - right, i.e. center of the right-hand edge of the bounding box
* `br` - bottom-right
* `b` - bottom, i.e. center of the bottom edge of the bounding box
* `bl` - bottom-left
* `l` - left, i.e. center of the left-hand edge of the bounding box
* `c` - center of the bounding box

Together a 'layoutref' and a 'locspec' denote a point in 2D user coordinates.

**Edge-based LocSpec** - as a special case of 'locspec', those locations
which define the edge of a layoutref (i.e. t,r,b,l) may be followed by an offset to
vary the resulting point position along the edge. The offset is separated from the
locspec by a colon (`:`), and may be either a number or a percentage.

Each edge starts at the 'left' (`t` / `b` edges) or 'top' (`l` / `r` edges) and
ends at the right/bottom of the edge respectively.

If a percentage is given (e.g. `:30%`), this represents that percentage along the
edge from the start. This implies that `#abc@t:0%` is equivalent to `#abc@tl`, and
`#abc@t:100%` is equivalent to `#abc@tr`. Note that the value given is not restricted
to 0%..100%, but can exceed this range.

If the value is a number rather than a percentage, it is treated differently. A
positive value is an offset from the start of the edge, while a negative offset
moves *backwards* from the end of the edge. This is analogous to slice indexing in
the Python language, where `a[-1]` represents the last item in the sequence `a`.

Edge offsets can be useful where many connector lines are joining an element and
it would be clearer to keep them separate; rather than having four connectors all
join an element at `@b` for example, consider joining them at `@b:20%`, `@b:40%`,
`@b:60%` and `@b:80%`.

**Direction Spec** - ('dirspec') denotes a directional relation between
two objects. The following dirspec values are supported:

* `h` - place horizontally to the right of the associated layoutref
* `H` - place horizontally to the left of the associated layoutref
* `v` - place vertically below the associated layoutref
* `V` - place horizontally above the associated layoutref

### Relspec

The above pieces fit together according to the following grammar.

```
dirspec    := : [h|H|v|V]
locspec    := @ [tl|t|tr|r|br|b|bl|l|c] | ([t|r|b|l] : length)
length     := number | (number %)

elref      := prevspec | ref
prevspec   := ^
ref        := # ident
ident      := alphanumeric
union      := [ elref { elref } ]

layoutref  := elref | union

relspec  := layoutref [dirspec | locspec]
```

Following the `relspec` as defined above, additional values may be given to define
margins or deltas.

When used as part of a dirspec (e.g. `#abc|H`), a single value defines the 'gap'
between the referenced layoutref and the element being positioned.

When used as part of a locspec (e.g. `@tl`), a pair of values may be provided which
define the `dx` and `dy` offsets to apply.

Some simple examples:

* `xy="#abc|h 5"` - position this element 5 units to the right of the element
  with `id="abc"`.
* `cxy="^"` - position this element to have its center on the center of the
  previous element.
* `xy="^|V"` - position this element directly above the previous element.
* `xy="^@br"` - position this element at the bottom-right of the
  previous element.
* `xy="#thing@tr 5 10"` - position this element at the top-right of
  element with `id="thing"`, offset by (5, 10).
* `xy="[#a #b]|v 2"` - position this element 2 units below the bounding box
  which contains both `#a` and `#b`.
* `width="[#a #b]"` - make this element as wide as the bounding box which
  contains both `#a` and `#b`.

A layoutref can be used anywhere an element reference is accepted for layout
purposes, including the `surround` and `inside` attributes. In the case of
`inside`, a union is treated as a rectangle.
