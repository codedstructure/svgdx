# Design Principles

1. The target is SVG 1.1, as defined by https://www.w3.org/TR/SVG11/. SVG 2 support is not intended due to (at present) poor support. Note that SVG 1.1 is the target - not any particular implementation or use case (e.g. web browsers are one target among many; HTML embedding cannot be assumed).
2. All valid SVG input must be preserved as-is; this is key to incremental improvement of both spec and implementation.
3. Limitations in scope are acceptable, even when no long-term plan seems viable to address them. For example, SMIL animation is unlikely to feature in use-cases considered by this project.
4. Where SVG has support for features, it should be used and expected to be used; the tool should not provide alternatives which provide minimal benefit over existing SVG support. For example centered text should be supported via appropriate `dominant-baseline` and `text-anchor` presentation attributes, rather than adjusted placement of text.
5. A "support library" should be developed, including symbols, CSS, and definitions (e.g. for arrow-head markers). Where reasonable, reference to library entities should be preferred to additional code in the tool.

# Target Feature List
* Given a basic `<svg>` element, derive reasonable attributes in the output including automatic `viewport`, derived from the bounding box of the contained elements.
* Ability to "watch" files and re-compile them on change, including at directory level.
* Relative positioning of 'sized' elements, including ability to 'distribute' elements linearly over a range.
* 'Hidden' elements, used for geometry reference, but not part of the output.
* "Geometric" scaling of symbols / groups.
* Attribute broadcast
* Automatic insertion of relevant blocks from library files.
* Inclusion by reference or inline of styles and other library entities.

# Concepts
* Bounding boxes for 'sized' elements (initially SVG's "Basic Shapes").
* Locations
* Uniform placement, eg xy, cxy, wh, etc
* Relative positioning - including percentage along a bounding box line
* Element aware attribute broadcasting
* ‘Standard library’
* Extensibility
* Ubiquitous text - every element can have attached text with context-dependent placement
