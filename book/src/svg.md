# Delta 0 - SVG

> SVG is the foundation for `svgdx`, and is not hidden away

## Overview

`svgdx` is a superset of [SVG](https://www.w3.org/TR/SVG11/). In that sense, any valid SVG is (in theory) already valid `svgdx` input, and being able to intersperse SVG with enhanced `svgdx` avoids providing too many limitations to `svgdx`. _In theory_, `svgdx` is at least as powerful as SVG - anything you can do in SVG, you can also do in `svgdx`. In practice there are various constraints[^1] which limit what can be done, though even these are more about being able to use the enhancements `svgdx` provides together with some `SVG` features - it's always possible to drop down to the lower level more widely if required.

## SVG is XML

If the benefit of having `svgdx` be a superset of SVG is that it provides enormous flexibility, the biggest downside is that SVG is an **XML** document format. Personally I quite like XML, or at least find its model of a hierarchy of tagged elements, each of which may have arbitrary attributes - and it's support for comments - compelling advantages over (for example) JSON. However, it is tedious to type, and the need to use XML entities for common characters such as `<`, `>` and quotes is frustrating.

For now `svgdx` is XML, though in future a non-XML syntax retaining equivalent semantics is a definite possibility.

### Elements and Attributes

SVG is built on a number of element types, each of which is parameterized through element-specific _attributes_.

`svgdx` provides new element types as well as additional attributes and semantics for existing SVG elements.

---

[^1]: for example the assumption that user-coordinates are used throughout.
