# Delta 2 - Shape Text

> Associating text with shapes is a fundamental diagramming technique

## Overview

SVG supports a number of different 'basic shapes', as well as support for complex 'path' elements. Text elements are also supported, but are entirely independent of other elements. Since associating text with shape elements is such a common part of making diagrams, `svgdx` has various tools to make this easier.

## The `text` attribute

Consider the following (not very interesting!) image:

![](./images/text-shape.svg)

This is generated from the following `svgdx` document:

```xml
{{#include ./images/text-shape.xml}}
```

The value of any `text` attribute is extracted and a new `<text>` element is created placed so it appears within the shape - centered by default.
