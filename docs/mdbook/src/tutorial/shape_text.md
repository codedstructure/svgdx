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
  <rect xy="50 0" wh="50" text="I am a square!"/>
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

## Text styling

## Standalone text elements
