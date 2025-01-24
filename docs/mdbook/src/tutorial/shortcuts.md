# Delta 1 - Shortcuts

> `svgdx` inherits its semantics from SVG, but aims to reduce verbosity and boilerplate

## Attribute shortcuts

A rectangle in SVG may be represented as follows:

```xml
<rect x="2" y="2" width="20" height="10"/>
```

Here there are four attributes - `x` and `y` specify _where_ the rectangle is placed, while `width` and `height` specify the _size_ of the rectangle.

`svgdx` allows combining these attributes into `xy` and `wh` respectively, so the same rectangle may be expressed in the shorter:

```xml
<rect xy="2" wh="20 10"/>
```

Note two things about this example:

* if the same value is present for both 'target' attributes, it may be specified just once; `xy="2"` means that both `x` and `y` attributes are given the value `"2"`.
* if multiple values are given, they may be separated by ['comma-whitespace'](https://www.w3.org/TR/SVG11/types.html#CommaWSP) - either a comma surrounded by optional whitespace, or whitespace alone.

Based on the above points, `"20"`, `"20,20"`, `"20, 20"` and `"20 20"` are all equivalent.

The attributes `xy` and `wh` are sufficient for rectangles, but other [basic shapes](https://www.w3.org/TR/SVG11/shapes.html) have other ways to specify their layout, and other shortcuts are available.

An SVG `<circle>` is positioned using the attributes `cx`, `cy` and `r`, which define the x and y positions of the center coordinate, and the circle's radius respectively. Analogous to the use of `xy` for the `<rect>` element, a circle may be positioned using the `cxy` shortcut attribute.

The set of attribute shortcuts are as follows:

| Attribute name | Meaning | Applies to |
|---|---|---|
| `xy` | Top-left coordinate[^1] | `rect`, `circle`, `ellipse` |
| `cxy` | Center coordinate | `rect`, `circle`, `ellipse` |
| `xy1` | First coordinate of a line | `line` |
| `xy2` | Second coordinate of a line | `line` |

## Root SVG Element Shortcuts

One of the frustrating things about hand-coding SVG documents is the set of attributes needed on the root `<svg>` element. As a minimum, these need to include the SVG version and namespace; a minimal SVG root element looks like:

```xml
<svg version="1.1" xmlns="http://www.w3.org/2000/svg">
```

Unfortunately even providing these two attributes isn't enough in most cases; there are more values applied at the root element level.

Consider the following image and corresponding SVG code:

```svgdx-xml
<svg version="1.1" xmlns="http://www.w3.org/2000/svg">
  <rect x="0" y="0" width="120" height="50" style="fill:red" />
  <rect x="120" y="0" width="120" height="50" style="fill:green" />
  <rect x="240" y="0" width="120" height="50" style="fill:blue" />
</svg>
```

Each of the three rectangles has the same size (120 x 100 ['user units'](https://www.w3.org/TR/SVG11/coords.html#Units)), but the displayed image (probably!) has the blue rectangle cut off and plenty of whitespace underneath the three coloured rectangles.
Most browser user agents display SVG images with a size of [300 x 150](https://svgwg.org/specs/integration/#svg-css-sizing) pixels if not otherwise specified.

As a convenience, `svgdx` translates an `<svg>` root element as follows:

* Any existing attributes are preserved.
* Default attributes for `version="1.1"` and `xmlns="http://www.w3.org/2000/svg"` are provided.
* The bounding box of all elements[^2] is computed, and `width`, `height` and `viewBox` attributes are set as appropriate to contain the full set of elements (plus a configurable border) regardless of their position in the coordinate space[^3].

When the above SVG is passed through `svgdx`, it outputs the following[^4]. Note how the addition of `width`, `height` and `viewBox` attributes cause the entire image - and only the image - to be rendered.

Using the shortcuts introduced so far, this SVG file can be created from the following input to `svgdx`:

```xml-svgdx
<svg>
  <config add-auto-styles="false"/>
  <rect xy="0" wh="120 50" style="fill:red"/>
  <rect xy="120 0" wh="120 50" style="fill:green"/>
  <rect xy="240 0" wh="120 50" style="fill:blue"/>
</svg>
```

## Summary

We've seen how `svgdx` allows less boilerplate to be written through the use of shortcut attributes which can expand to multiple attributes, and how the tedious job of determining the root SVG element is eliminated entirely in many cases.

One outcome of having shortcuts available is there may be several ways to express the same concept. The most concise is not always the most understandable, and if both a shortcut and a more explicit instruction are present, the more explicit instruction should always take priority.

The theme of being able to express more with less will return as we continue looking at `svgdx`, and we'll see even more concise (and - hopefully - clear) ways that the given image can be authored.

---

[^1]: While `xy` normally refers to the top-left coordinate, this can be modified using the `xy-loc` attribute - see the [positioning](./positioning.md) chapter for more.

[^2]: Bounding box calculation may not be evaluated perfectly for all SVG elements - e.g. `svgdx` cannot determine the exact size of rendered text elements, and the extension of certain arcs in `<path>` elements is not considered.

[^3]: As with all geometry processing in `svgdx`, the assumption is that user coordinates are used, i.e. without suffixes such as `em`, `pt`, `mm` etc.

[^4]: Whitespace has been added for clarity.
