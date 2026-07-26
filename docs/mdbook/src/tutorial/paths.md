# Delta 6 - Paths

> SVG `<path>` elements gain additional features in svgdx.

## Overview

Many SVG documents are composed solely of path elements, which are very
flexible: basic shapes (rectangles, circles, lines, polygons, etc) can all be
implemented using the `path` element, and it is often more compact.

`svgdx` supports the following enhancements to `path` statements:

* bearing commands, allowing relative and absolute angled lines to be drawn
* repeated command fragments
* determining points at intervals along a path

These are covered in the following.

## Bearing commands

Early editions of SVG 2 proposed a 'bearing' command for path elements. This was
later removed, but svgdx translates bearing commands to supported SVG 1.1 path
commands.

The following extract is derived from an [early draft](https://www.w3.org/TR/2015/WD-SVG2-20150409/paths.html#PathDataBearingCommands) of the SVG2 working draft with minimal changes.

---

The bearing commands (B or b) set the current bearing, which influences the orientation of subsequent relative path commands:

<table>
<thead><tr><th>Command</th><th>Name</th><th>Parameters</th><th>Description</th></tr></thead>
<tr><td><strong>B</strong> (absolute)
<strong>b</strong> (relative)</td><td>Name</td><td>angle+</td><td>
Sets the current bearing. The parameter is an angle in degrees, where 0 indicates the direction of the positive x-axis. B (uppercase) sets the current bearing to the specified angle; b (lowercase) sets the current bearing to be the angle of the tangent at the end of the preceding path command plus the specified angle. The current point is unaffected. Although multiple parameters may be specified, this usually will not be useful, as they could be combined into a single angle value.</td></tr>
</table>

The example below shows how bearing commands can be used to draw a regular pentagon.

```xml-svgdx
<svg>
  <path style="fill:#eee; stroke:deeppink; stroke-width:8px; stroke-linejoin: round;"
        d="M 150,10
           B 36 h 47
           b 72 h 47
           b 72 h 47
           b 72 h 47 z"/>
</svg>
```

Image showing the use of the bearing command.

Bearing commands can be used to position the end points of the sides of a regular polygon without having to use trigonometry to calculate them based on the polygon's interior angles.

---

## Repeats

In addition to the 'bearing' commands, svgdx provides a 'repeat' command. Together with the existing path commands, basic [turtle graphics](https://en.wikipedia.org/wiki/Turtle_graphics) may be implemented within a single `<path>` element.
The 'repeat' command causes the repeated instructions to be expanded within the path 'd' attribute itself, rather than creating new path elements as would be done with [loops](./loops_conditions.md).

### Syntax

 **r** *N* **[** commands **]**

  *N*: integer; number of types to repeat *commands*.

Note that repeat commands may be nested, but there is a configurable limit (`path-repeat-limit`) defaulting to 10000 to prevent excessively large path elements being generated using large or deeply nested repeat counts.

### Example

```xml-svgdx
<svg>
  <line xy="0" width="100"/>
  <line xy="^" dy="10" width="^"/>
  <path d="M0 0 r10[l5 10 5 -10]" class="d-red"/>
</svg>
```

## Line offsets

Lines in svg have a (perhaps implied) start and end point; for `<line>` elements this is the (resolved) `x1` and `y1` coordinates, and for `<polyline>` and `<path>` elements the `points` / `d` sequences will be evaluated from left to right.

Note this applies to all line-like elements, including [connectors](connectors.md).

```xml-svgdx
<svg>
  <path id="p1" d="M0 0 q30 0 30 30" style="stroke-width: 1"/>
  <circle cxy="#p1:0%" r="1"/>
  <circle cxy="#p1:33.33%" r="1"/>
  <circle cxy="#p1:66.67%" r="1"/>
  <circle cxy="#p1:100%" r="1"/>
</svg>
```
