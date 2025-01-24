# Delta 4 - Positioning

> `svgdx` provides alternatives to the absolute positioning of elements provided by SVG

## Overview

Most SVG elements are placed on a coordinate grid using absolute values within a defined coordinate system. An exception to this is the `<tspan>` element, which naturally "follows on" in terms of position from the previous `<tspan>` element. Being able to do this (and more) would be useful for other SVG elements, and is provided by `svgdx`.

Two important notes should be considered when planning positioning in `svgdx`:

* User units should be used throughout; absolute units (e.g. those with some measurement suffix, such as `px` or `mm`) will prevent `svgdx` understanding the positions of elements.
* `svgdx` diagrams are 'expected' to be between approximately 10 and 1000 units in each dimension. While there are no hard limits on size, various aspects make assumptions about appropriate absolute values - such as default text or arrow-head size - which won't be valid with very small or very large drawings. SVG is by nature scalable, and scaling the largest dimension to fit in this range should generally be feasible.

## Simple relative positioning

The simple cases of 'after the previous element' and 'below the previous element' which `<tspan>` handles automatically for text are dealt with generically in `svgdx` through special cases of the `xy` attribute.

| `xy` attribute value | meaning |
|---|---|
| "^\|h" | to the right of ('horizontally after') the previous element |
| "^\|H" | to the left of ('horizontally before') the previous element |
| "^\|v" | below ('vertically after') the previous element |
| "^\|V" | above ('vertically before') the previous element |

For each of these, a further numeric value can be given which provides the 'margin' before the next element starts.

So we can have:

```svgdx-xml-inline
<svg>
 <rect xy="0" wh="20" text="a"/>
 <rect xy="^|h" wh="20" text="b"/>
 <rect xy="^|v" wh="20" text="c"/>
 <rect xy="^|h" wh="20" text="d"/>
</svg>
```

or:

```svgdx-xml-inline
<svg>
 <rect xy="0" wh="20" text="A"/>
 <rect xy="^|h 10" wh="20" text="B"/>
 <rect xy="^|V 5" wh="20" text="C"/>
 <rect xy="^|H 10" wh="20" text="D"/>
</svg>
```

## Layout

The most important concept for positioning is the element **bounding box**. This is an axis-aligned rectangle which is the minimum size required to cover a shape. For (non-rotated) `<rect>` elements, the bounding box is identical with the element's own layout; for other shapes it will there will usually be some area inside the bounding box that is not within the shape itself.

The diagram below shows the bounding box (blue dashed line) of several shapes (in red).

```svgdx
<svg>
 <rect id="r1" xy="0" wh="30 20" class="d-none d-fill-red"/>
 <circle id="c1" cxy="50 10" r="10" class="d-none d-fill-red"/>
 <polyline id="pl1" points="60,30 40,50 60,70" class="d-red"/>
 <polygon id="pg1" points="10,30 30,50 30,60 10,70 0,60 0,40" class="d-none d-fill-red"/>

 <rect surround="#r1" class="d-blue d-dash"/>
 <rect surround="#c1" class="d-blue d-dash"/>
 <rect surround="#pl1" class="d-blue d-dash"/>
 <rect surround="#pg1" class="d-blue d-dash"/>
</svg>
```

Each bounding box has nine 'locations' which can be used as relative positioning points, as shown here:

```svgdx
<svg>
<config font-size="4" font-family="monospace"/>
<defaults><circle style="stroke: none; opacity:0.9;" r="3"/></defaults>
<rect id="r1" wh="50" class="d-fill-beige"/>
<circle cxy="#r1@tl" text="tl"/>
<circle cxy="#r1@t" text="t"/>
<circle cxy="#r1@tr" text="tr"/>
<circle cxy="#r1@r" text="r"/>
<circle cxy="#r1@br" text="br"/>
<circle cxy="#r1@b" text="b"/>
<circle cxy="#r1@bl" text="bl"/>
<circle cxy="#r1@l" text="l"/>
<circle cxy="#r1@c" text="c"/>
</svg>
```

A mnemonic to remember these positions is "TRBL", so stay out of 'trouble' by remembering these! A further point to note is that for the corner positions, the Top/Bottom indicator is always before the Left/Right indicator, so it's always `br` - not `rb` - for the bottom-right corner.
