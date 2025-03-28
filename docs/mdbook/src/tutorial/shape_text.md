# Delta 2 - Shape Text

> Associating text with shapes is a fundamental diagramming technique

## Overview

SVG supports a number of different 'basic shapes', as well as support for complex 'path' elements comprising line and curve segments. Text elements are also supported, but are entirely independent of other elements. Since associating text with shape elements is such a common part of making diagrams, `svgdx` has various tools to make this easier.

## The `text` attribute

Consider the following (not very interesting!) image and source:

```svgdx-xml
<svg>
  <rect xy="0" wh="40 10" text="I am a rectangle!"/>
  <circle cxy="20 30" r="15" text="I am a circle!"/>
  <rect xy="50 0" wh="40" text="I am a square!"/>
</svg>
```

The value of any `text` attribute is extracted and a new `<text>` element is created placed so it appears within the shape - centered by default.

The first (rectangle) element above is converted by svgdx into the following SVG fragment - the base element is immediately followed by a new text element, with content generated from the `text` attribute and positioned appropriately.

```xml
<rect x="0" y="0" width="40" height="10"/>
<text x="20" y="5" class="d-text">I am a rectangle!</text>
```

Note the use of the `d-text` class; CSS is used to anchor text as required. For centered text, the anchor is set to the center of the text itself, rather than the bottom-left default.

## Multi-line text

The text attribute works well where there is a single short text value - perhaps a label - attached to another element. If the text is longer, squeezing into a single attribute could be messy. There are several ways of dealing with this:

```xml-svgdx
<svg>
  <rect id="r" xy="0" wh="50 20" text="One way to implement multiple
line text - by splitting the `text`
attribute over several lines."/>

  <rect xy="^|h 5" wh="50 20" text="Or use '\\n'\nto separate\nlines"/>

  <rect xy="#r|v 5" wh="50 20">
    And you can just
    put the text as
    the element content.
  </rect>

  <rect xy="^|h 5" wh="50 20">
<![CDATA[The content can be a CDATA
  block, allowing things such
  as "i < &j" without the need
  for escaping.
]]>
  </rect>
</svg>
```

## Text positioning

Text may be positioned within a shape using the `text-loc` attribute.

```xml-svgdx
<svg>
  <rect id="tl" wh="25 15" text-loc="tl" text="top-left"/>
  <rect xy="^|h 2" wh="^" text-loc="t" text="top"/>
  <rect xy="^|h 2" wh="^" text-loc="tr" text="top-right"/>
  <rect xy="#tl|v 2" wh="^" text-loc="bl" text="bottom-left"/>
  <rect xy="^|h 2" wh="^" text-loc="b" text="bottom"/>
  <rect xy="^|h 2" wh="^" text-loc="br" text="bottom-right"/>
</svg>
```

Multi-line text is aligned as appropriate based on its position; text positioned at the right will be right-aligned etc.

```xml-svgdx
<svg>
<rect wh="20" text-loc="br" text="several\nlines of\ntext"/>
</svg>
```

As shown above in all the examples above, by default text is positioned _within_ the associated shape. It may also be placed _outside_, and this is the default for text associated with lines (where 'inside' doesn't make a lot of sense), but can also be triggered using the `d-text-outside` class:

```xml-svgdx
<svg>
  <line x1="0" y1="0" width="20" text-loc="t" text="above"/>
  <line x1="0" y1="10" width="20" text-loc="b" text="below"/>
</svg>
```

```xml-svgdx
<svg>
  <rect wh="10" text-loc="r"
        text="right" class="d-text-outside"/>
  <box wh="30 10" _="avoid text clipping"/>
</svg>
```

> The example above shows a current limitation of `svgdx`: it does not
> compute a bounding box for text objects. (Without a rendering context or
> font-handling, it can't know exactly how big any text will end up being.)
>
> This can result in text outside any other object being clipped in the
> generated root SVG viewBox calculation; use of the `<box>` element to
> force a larger canvas without generating any further SVG elements is an
> effective workaround.


## Text styling

When a `text` attribute is provided on a shape element, an additional `<text>` element is created. Many of the classes and attributes of the source element are copied across, but _not_ the `style` attribute.
This is because with common style presentation attributes such as `fill` or `stroke`, the text should generally have a different style to the containing element. Having red-filled text within a red-filled rectangle would be unhelpful.

For this reason a separate `text-style` attribute is available, which is injected into the newly created text element.

```xml-svgdx
<svg>
  <rect wh="20" style="fill: green"
        text-style="fill: yellow; stroke: red; stroke-width: 1"
        text="very\nstylish..."/>
</svg>
```

There are other attributes and classes which affect text styling. In general, standard SVG attributes which apply as presentation attributes to `<text>` elements (and not shapes in general) may be provided on the source element and will be transferred to the new `<text>` element.
Examples of such attributes include `font-weight`, `font-family` and `letter-spacing`.

A variety of use-cases are covered by text-specific auto styles.

### Font size

`d-text-smallest` / `-smaller` / `-small` / `-medium` / `-large` / `-larger` / `-largest`

These styles control the size of text. The default text size is `d-text-medium`, but providing
this style as an option allows the various relative size styles to be used if global font-size
is overriden.

```xml-svgdx
<svg>
<rect wh="30 4" text="smallest" class="d-text-smallest"/>
<rect xy="^|v" wh="^" dh="125%" text="smaller" class="d-text-smaller"/>
<rect xy="^|v" wh="^" dh="125%" text="small" class="d-text-small"/>
<rect xy="^|v" wh="^" dh="125%" text="default"/>
<rect xy="^|v" wh="^" dh="125%" text="large" class="d-text-large"/>
<rect xy="^|v" wh="^" dh="125%" text="larger" class="d-text-larger"/>
<rect xy="^|v" wh="^" dh="125%" text="largest" class="d-text-largest"/>
</svg>
```

### Font styles

Monospace, italic and bold text faces are available through the classes
`d-text-monospace`, `d-text-italic` and `d-text-bold` respectively. These may be combined as required.

```xml-svgdx
<svg>
<rect wh="15" text="normal"/>
<rect xy="^|h" wh="^" text="bold" class="d-text-bold"/>
<rect xy="^|h" wh="^" text="italic" class="d-text-italic"/>
<rect xy="^|h" wh="^" text="bold\nitalic" class="d-text-bold d-text-italic"/>
<rect xy="^|h" wh="^" text="mono"
    class="d-text-monospace"/>
<rect xy="^|h" wh="^" text="mono\nbold"
    class="d-text-monospace d-text-bold"/>
<rect xy="^|h" wh="^" text="mono\nitalic"
    class="d-text-monospace d-text-italic"/>
<rect xy="^|h" wh="^" text="mono\nbold\nitalic"
    class="d-text-monospace d-text-bold d-text-italic"/>
</svg>
```

### Pre-formatted text

Pre-formatted text is useful for code listings, or other cases where whitespace is significant and should be preserved.

This style is similar to `d-text-monospace`, but in addition the text element has spaces replaced with non-breaking spaces. This prevents the usual XML whitespace collapse which replaces multiple contiguous spaces with a single space.

> The NBSP replacement approach may change in future,
> as SVG2 has better support for preserving whitespace.

```xml-svgdx
<svg>
<rect wh="40 15" class="d-text-pre"
  text="def square(x):\n    return x * x"/>
</svg>
```

### Vertical text

Text may be oriented vertically using the `d-text-vertical` class.

```xml-svgdx
<svg>
  <rect wh="6 15" text="Hello" class="d-text-vertical"/>
</svg>
```

## Standalone text elements

While text within elements can be useful, there is still a place for the `<text>` element on its own.

An example use is to provide _multiple_ text objects associated with one shape.

```xml-svgdx
<svg>
  <box wh="40 30"/>
  <rect id="r" cxy="^" wh="15"/>
  <text xy="#r@t" text="Top"/>
  <text xy="#r@r" text="Right"/>
  <text xy="#r|v" text="Bottom"/>
  <text xy="#r|H" text="Left"/>
</svg>
```

Note that both 'dirspec' and 'locspec' formats can be provided, and text anchors are computed appropriately.

In the case of separate `<text>` elements, the default associated position is *outside* the referenced object; analogous to the `d-text-outside` class, there is a `d-text-inside` class to override this default:

```xml-svgdx
<svg>
  <defaults>
    <text class="d-text-inside"/>
  </defaults>
  <rect id="r" wh="25"/>
  <text xy="#r@t" text="Top"/>
  <text xy="#r@r" text="Right"/>
  <text xy="#r|v" text="Bottom"/>
  <text xy="#r|H" text="Left"/>
</svg>
```
