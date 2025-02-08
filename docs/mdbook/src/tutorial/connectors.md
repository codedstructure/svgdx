# Delta 5 - Connectors

> Lines between shapes provide valuable information in many diagrams

## Overview

Connections between elements are a key part of diagrams, where they can represent data flow, dependencies, or other associations. In `svgdx` connectors link together a `start` and `end` element, and use [auto-styles](./auto_styles.md) to provide visual information such as directionality.

## Simple Connectors

Given two elements with unique `id` attributes, a connection between them may be created using the `<line>` or `<polyline>` element with `start` and `end` attributes of the relevant id references:

```svgdx-xml-inline
<svg>
  <rect id="a" wh="20 10" text="input" />
  <rect id="b" xy="^|h 10" wh="^" text="output" />

  <line start="#a" end="#b"/>
</svg>
```

In this simple form, a straight line between the two connected shapes is always drawn.
If only base element references (i.e. `#abc`) are given for the start and end points, the location the line is drawn from is calculated automatically based on the shortest connection:

```svgdx-xml-inline
<svg>
  <rect id="a" wh="20 10" text="input" />
  <rect id="b" xy="^ 25" wh="^" text="output" />

  <line start="#a" end="#b"/>
</svg>
```

This can result in connectors which don't look great, e.g. the following is not particularly pleasing:

```svgdx-xml-inline
<svg>
  <rect id="a" wh="20 10" text="input" />
  <rect id="b" xy="^ 20" wh="^" text="output" />

  <line start="#a" end="#b"/>
</svg>
```

To counter this, provide more explicit start and(/or) end references, e.g. using explicit locations or edge-specs.

```svgdx-xml-inline
<svg>
  <rect id="a" wh="20 10" text="input" />
  <rect id="b" xy="^ 20" wh="^" text="output" />

  <line start="#a@r" end="#b@t"/>
</svg>
```

Providing an "edgespec", which consists of one of the edges (`t` for top, `r` right, `b` bottom, or `l` for left) followed by a colon and a percentage or offset - for example `#abc@r:25%` - can be particularly useful when multiple connections would otherwise target the same point:

```svgdx-xml-inline
<svg>
  <rect id="a" wh="10" text="a" />
  <rect id="b" xy="^|h 5" wh="10" text="b" />
  <rect id="c" xy="^|h 5" wh="10" text="c" />
  <rect id="d" xy="^|v 5" wh="10" text="d" />
  <rect id="z" xy="#b|v 20" wh="10" text="z" />

  <line start="#a" end="#z@t:20%"/>
  <line start="#b" end="#z@t:40%"/>
  <line start="#c" end="#z@t:60%"/>
  <line start="#d" end="#z@t:80%"/>
</svg>
```

## Connector styles

Classes can be applied to connector elements which control the style. Note these are not restricted to connectors,
but this is their primary use-case. As with classes in general, these can be combined as appropriate.

```svgdx
<svg>
  <defaults><line text-lsp="1.5"/></defaults>
  <rect id="a" wh="20 5" text="start" />
  <rect id="b" xy="^|h 30" wh="20 5" text="end" />
  <line start="#a" end="#b" class="d-arrow" text="d-arrow" text-dy="-2"/>

  <rect id="a2" xy="#a|v 7" wh="20 5" text="start" />
  <rect id="b2" xy="^|h 30" wh="20 5" text="end" />
  <line start="#a2" end="#b2" class="d-biarrow d-red" text="d-biarrow\nd-red"/>

  <rect id="a3" xy="#a2|v 7" wh="20 5" text="start" />
  <rect id="b3" xy="^|h 30" wh="20 5" text="end" />
  <line start="#a3" end="#b3" class="d-dash d-arrow" text="d-dash\nd-arrow"/>
</svg>
```

## Polyline connectors

Connectors can be defined by the `<polyline>` SVG element in addition to `<line>`.
With polylines, connectors are restricted to horizontal and vertical segments, with corners at appropriate places.

> Currently svgdx only supports polyline connectors with one or two corners.

```svgdx
<svg>
  <rect id="a" wh="20" text="#a@r" text-loc="r"/>
  <rect id="b" xy="^|h 30" dy="10" wh="20" text="#b@l" text-loc="l"/>
  <text xy="#a@t" text="a" class="d-text-bold"/>
  <text xy="#b@t" text="b" class="d-text-bold"/>

  <line start="#a@r" end="#b@l" text="line" text-dy="-4"/>

  <rect id="c" xy="#a|v 15" wh="20" text="#c" text-loc="r"/>
  <rect id="d" xy="^|h 30" dy="10" wh="20" text="#d" text-loc="l"/>
  <rect id="e" xy="^|h 10" dy="-20" wh="20" text="#e" text-loc="t"/>
  <rect id="f" xy="^|h 10" dy="-20" wh="20" text="#f" text-loc="l"/>
  <text xy="#c@t" text="c" class="d-text-bold"/>
  <text xy="#d@t" text="d" class="d-text-bold"/>
  <text xy="#e@t:25%" text="e" class="d-text-bold"/>
  <text xy="#f@t" text="f" class="d-text-bold"/>

  <polyline start="#c" end="#d" text="polyline" text-dy="-8"/>
  <polyline start="#e" end="#f" text="polyline"/>
</svg>
```

Note that with the polyline case, the start and end specs don't need the full locspec (i.e. `@r`, `@l`), as corner
locations are not included when evaluating default join points on elements.

By default, corners happen 50% along the path between the connected elements,
but this can be overwritten with the `corner-offset` attribute.

```svgdx
<svg>
  <rect id="a" wh="20" text="#a" text-loc="r"/>
  <rect id="b" xy="^|h 50" dy="10" wh="20" text="#b" text-loc="l"/>
  <text xy="#a@t" text="a" class="d-text-bold"/>
  <text xy="#b@t" text="b" class="d-text-bold"/>

  <polyline start="#a" end="#b" corner-offset="25%" text='polyline\ncorner-offset="25%"' text-dxy="5 -3" text-lsp="1.5"/>

  <line xy1="#a@r:120%" text="25%" text-loc="b" width="{{(#b~x1 - #a~x2) * 0.25}}" class="d-biarrow d-thin d-dash"/>
  <line xy1="^@r" text="75%" text-loc="b" width="{{(#b~x1 - #a~x2) * 0.75}}" class="d-biarrow d-thin d-dash"/>
</svg>
```

For elbow connectors such as the above, the corner-offset can be given as either a percentage or an absolute value.
If the connector is joining things facing the same direction, it requires an absolute value, which is 3 units by default.

```svgdx
<svg>
  <rect id="a" wh="20" text="#a@t" text-loc="t"/>
  <rect id="b" xy="^|h 50" dy="10" wh="20" text="#b@t" text-loc="t"/>
  <text xy="#a@tl" text="a" class="d-text-bold"/>
  <text xy="#b@tr" text="b" class="d-text-bold"/>

  <polyline start="#a@t" end="#b@t" corner-offset="12" text='polyline\ncorner-offset="12"' text-loc="t" text-lsp="1.5"/>
  <line xy2="#a@t:80%" height="12" text="12" text-loc="r" class="d-biarrow d-thin d-dash"/>
</svg>
```
