# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Support for `<spec>` and `<reuse>` custom elements to allow templating of
  elements in the rendered document.

## [0.4.2 - 2024-02-04]

- Added: initial support for `<path>` bounding boxes. Note this currently ignores
  any curves (cubic, quadratic, arcs) and considers only visited endpoints of
  these shapes.

- Changed approach to indentation so this now works consistently with elements
  processed out-of-order (e.g. geometry defined in terms of elements occurring
  later in the document)

- Added a `<config>` element which can define config options within an input
  document. See the [element docs](docs/elements#config) for more info. This
  allows the [examples/refresh.sh](examples/refresh.sh) script to be run
  cleanly over all the examples.

## [0.4.1 - 2024-01-22]

- Improved error handling, with fewer panics caused by invalid input.

- Added a check that the output won't override the input file (after being
  bitten by exactly this...)

- Updated examples to use `.xml` / `.svg` for input and output filenames
  respectively, rather than `-in.svg` / `-out.svg`. While the auto-completion
  for input files which `.svg` might provide in some editors is useful,
  the extension lies about it being a valid SVG file which can be confusing,
  e.g. when viewing input source for examples in GitHub.

- Update shlex to address RUSTSEC-2024-0006 (no relevant parts of the affected
  API are used in svgdx.)

## [0.4.0] - 2024-01-15

- Changed: auto-styles are no-longer suppressed when user-defined styles / defs
  are present. Note this may require changing user-styles to increase priority
  over auto-styles.

- Added: Additional values for scalar specs, including `cx`, `cy`, `rx`, `ry`.
  Existing scalar values can now be referenced by alternate names, e.g. `width`
  in addition to `w` or `y` in addition to `t`. The intent here is that SVG
  attribute names (e.g. `y` or `height` for a rectangle) are used as scalar names.

- Added: support for out-of-order references in `surround` and other contexts.

  Since `surround` is intended to support 'background' fills around shapes,
  it needs to be painted before the shapes in contains - and therefore
  reference elements occurring later in the document (which may in turn
  reference other elements later in the document to establish placement).

  A more general approach to these 'recursive' references is implemented,
  though it may be slow in the general case with large documents.
  The previous `populate()` stage followed by 'simple' / 'not-simple' calls
  to `expand_attributes()` have been replaced by repeated 'process remaining
  elements which couldn't be handled' stages until success (no further
  elements) or stall (couldn't reduce the number of remaining elements).

  NOTE: one (temporary) limitation of the approach here is the generated
  indentation/newline placement is less consistent, as output elements are
  no longer processed in document order.

- Internal: Made fields of `TransformerContext` private with appropriate
  access methods, e.g. `elem_map.get()` -> `get_element()`. Evaluation functions
  now take a `&TransformerContext` where previously they took `&HashMap`s etc
  corresponding to the internal types.

- Internal: `pop_idx` / `insert_idx` functions on `SvgElement`, to allow an
  attribute to be removed and replaced 'in sequence' with multiple other
  attributes without having to iterate through all existing attributes and
  rebuild a new `AttrList`. NOTE: This isn't a nice API and may change.

- Internal: switch to using `lazy_regex` for performance.

## [0.3.1] - 2024-01-04

- Fixed: expressions including a variable as e.g. "20 24" would become "2024" with
  whitespace collapsed. When used in numeric contexts this would create problems.
  It now remains as the text value "20 24" as intended.
- Fixed: panic resolving element size with `dwh` (etc) when `wh` referred to a
  variable or other expression.

## [0.3.0] - 2024-01-03

- Changed: more consistent public API; `get_config()` and `run()` are still top-level
  functions, but the various `transform_*` functions allow a range of input/output options
  for processing documents.
- Changed: split `svgdx::TransformConfig` out of `svgdx::Config` to handle per-transform
  settings, leaving top-level `Config` for 'front-end' options from the `svgdx`
  command-line program.
- Changed: default styles now use `fill: white` for shapes to improve appearance of
  overlapping shapes. Shapes using `surround` still have no fill via a `d-surround` class.
- Added: additional command-line options to tweak transformation:
  - `--debug` to include more debugging info in the generated document
  - `--scale` to allow scales other than 1 user-unit == 1mm
  - `--border` to specify width of border around entire image
  - `--no-auto-style` to prevent svgdx automatically adding style/defs entries
  - `--background` to specify background colour (default none)
- Fixed: Blank lines in multi-line text are now rendered correctly.
- Fixed: Move `dwh` handling to ensure it works with `cxy`.

## [0.2.0] - 2024-01-01

- Added: Initial support for `<path>` elements, though currently very limited.
- Changed: single value refspec is now #abc.x to avoid #abc.h (height of #abc) conflicting
  with #abc:h (position horizontally on the right of #abc). Mnemonic: '.' has a single dot
  and returns a single numeric value...
- Add support for locspecs in `points` attribute for polyline and polygon.
- Add `dx`/`dy`/`dxy` support for `points` values, translating each point in turn.
- Added `d-thin` and `d-biarrow` auto-style classes.
- Add support for dirspec (h/H/v/V) relative positioning from a referenced element,
  e.g. `xy="#abc:h"`
- Add various auto-style classes: `d-{colour}` & `d-fill-{colour}` for various colours,
  `d-dot` / `d-dash` for stroke styling, `d-arrow` for arrowhead marker, etc.
- Added `surround` attribute for rect / circle / ellipse which takes a list of
  element references and creates a shape surrounding them; a `margin` attribute
  can provide x & y margins expanding the surrounding shape from the minimal
  bounding box enclosed.

## [0.1.0] - 2023-12-25
- Initial public release. Happy Christmas!
