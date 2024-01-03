# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
