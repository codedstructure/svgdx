# Delta 5 - Connectors

> Lines between shapes provide valuable information in many diagrams

## Overview

Connections between elements are a key part of diagrams, where they can represent data flow, dependencies, or other associations. In `svgdx` connectors link together a `start` and `end` element, and use [auto-styles](./auto_styles.md) to provide visual information such as directionality.

## Simple Connectors

Given two elements with unique `id` attributes, a connection between them may be created using the `<line>` or `<polyline>` element with `start` and `end` attributes of the relevant id references:

```svgdx-xml
<svg>
  <rect id="in" wh="20 10" text="input" />
  <rect id="proc" xy="^|h 10" wh="^" text="process" />
  <rect id="out" xy="^|h 10" wh="^" text="output" />

  <line start="#in" end="#proc" class="d-arrow"/>
  <line start="#proc" end="#out" class="d-arrow"/>
</svg>
```
