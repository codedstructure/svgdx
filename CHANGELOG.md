# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- **Breaking format changes**:
  - the separator for relative positioning has changed from ':' to '|'.
    Example: `xy="#abc:h"` becomes `xy="#abc|h"`.
  - the separator for elref scalars has changed from '.' to '~'.
    Example: `width="#other.h"` becomes `width="#other~h"`.

  This change is to better align with XML, where 'id' attribute names are
  explicitly allowed to include the '.' and ':' characters.
  In future ':' may be used as part of the 'elref' target format (e.g.
  `#abc:def` would meaningfully reference a particular element).

  Updating svgdx documents with case-sensitive search-and-replace for
  `:x` -> `|x` for each `x` in `[hHvV]` is a good start the relpos change;
  the scalarspec change requires a bit more care.

- Added: support for line elements to be specified with `width` / `height` attrs,
  e.g. `<line xy1="#abc@r" width="10"/>`.

- Changed (infra): only tags (with leading 'v') get pushed to the online editor at
  [svgdx.net](svgdx.net). Tagging format has changed from `0.0.1` to `v0.0.1` to
  support this (i.e. allow non-release tags to avoid pushing to github pages...)

- Docs are now in mdbook format. Note they rely on [mdbook-svgdx](https://github.com/codedstructure/mdbook-svgdx)
  for rendering embedded diagrams; see [docs/mdbook/README.md](docs/mdbook/README.md)
  for more info.

## [0.17.1 - 2025-01-24]

- Added: `text` attributes now processed for `<box>` and `<point>` elements.

- Fixed: relative positioning of `<use>` elements where the target bbox was not at
  the origin.

## [0.17.0 - 2025-01-18]

- Added: `<for>` loops: `<for data="$list" var="n" [idx-var="idx"]>`, where the
  `var` variable is updated with each entry in `list` at each iteration.
  See the [for-loop.xml](examples/for-loop.xml) example.

- Added: expression lists now support string values as well as numbers, and various
  string functions have been added, including `split`, `splitw`, `join`, `trim`.
  Existing functions such as `if`, `select`, `eq`/`ne` which previously only worked
  on numbers now also support strings where appropriate. Note that string values will
  be surrounded by single quotes - to remove these and unescape things when using as
  e.g. `text` attribute, use the `_(...)` function, which converts the 'string' value
  into normal text.

- Added: `divmod(x, y)` function. Returns the pair (x // y, x % y).

- Added: `<defaults>` container element to provide attribute and class defaults for
  matching elements. See [examples/defaults.xml](examples/defaults.xml) and the
  [docs](docs/elements.md#defaults) for more information.

- Improved: grid auto-styles are now parameterized; `d-grid5` / `d-grid10` are gone
  (`d-grid` remains as-is) and new `d-grid-N` where N is any integer from 1 to 100
  creates an appropriately dimensioned grid. The `d-hatch`, `d-crosshatch` and
  `d-stipple` classes are parameterized in the same way. New classes `d-grid-v` and
  `d-grid-h` (also parameterizable) provide vertical / horizontal grid lines.

- Editor: Fixed SVG download button to make a fresh request without metadata for
  a cleaner download file. Also removed the base64 data URI copy, replacing with
  simple copy of the generated SVG.

- svgdx-server: update axum package to 0.8.1

## [0.16.0 - 2024-12-31]

- Added: auto styles for text-outlines. These use the SVG2 `paint-order` attribute
  to ensure the fill is painted after the stroke, and use `stroke-width` to create
  an appropriate text outline. This is available through the class `d-text-ol`,
  which applies a default text outline, as well as the more specific `d-text-ol-[colour]`
  and `d-text-ol-[width]`, where `colour` is an SVG colour name and `width` is one
  of `thinner`, `thin`, `medium` (the default), `thick` or `thicker`.

  As part of supporting clean text outlines, `stroke-linecap` and `stroke-linejoin`
  styles are both set to `round` for everything; this generally results in tidier
  output where separate line objects join at the same point, compared to the SVG
  defaults of `butt`/`miter`, but has a noticeable effect on `stroke-dasharray`,
  so the `d-dot` / `d-dash` / `d-flow` (and a new stroke pattern `d-dot-dash`) have
  been updated to match. In particular, `d-dot` now renders as a series of circular
  dots rather than small squares.

- Added: new config option `svg-style` available as `TransformConfig::svg_style`
  or `--svg-style` command line argument, inserted as `style` attribute on the
  root SVG. Use-case is to provide inline style when embedded in a larger document.

- Added: `<box>` element. Analogous to the `<point>` element, but defining a
  'phantom' rectangle which may be used for relative positioning.

- Internal: replaced regex with basic string parsing throughout, eliminating
  a dependency and reducing size of WASM build.

- Improved: refactor to consolidate element reference parsing; improves
  consistency, allowing e.g.:
  - edge spec to be used in connectors
  - allowing 'previous element' references (`^`) in contexts which
    previously only supported id-based references, including
    connector `start` / `end` attributes.
  - element id references may use the '-' character

- Fixed: `text-style` values now applied to `<tspan>` as well as `<text>` elements.

- Fixed: avoid use of CSS `:not()` pseudo selector, not supported in many SVG
  contexts (e.g. Inkscape)

- Fixed: substituting multiple auto-style classes into an element via variable
  expansion.

- Fixed: `<path>` element without `d` attr is no longer an error when computing bbox.

- Fixed: merging auto-style defs/style elements with any existing such elements was
  broken, and has been abandoned as not worth the hassle.

- Fixed: bbox calculation for `<g>` elements with `transform` attributes.

- svgdx-server: add basic CLI options to specify listening address & port,
  as well as an `--open` flag to open a web browser with the svgdx editor.

- Editor: limit zoom range

- Editor: add SVG2 help link

## [0.15.1 - 2024-12-15]

- Changed: '%' operator now computes non-negative result, as in the Python
  '%' operator rather than the Rust '%' operator. This only affects negative
  operands, and is more useful when using '%' to compute indices into lists.
  Previous behaviour was `-1 % 4 == -1`; this change means `-1 % 4 == 3`.

- Fixed: precedence of the unary minus operator was incorrect. This fixes
  expressions such as `-5 - 8` (now `-13` rather than `3`).

- Improved: `inside` attribute behaviour for inscribing circles / ellipses.
  Still room for further improvements here, but simple cases (e.g. ellipse
  inside rect) now work as expected.

- Fixed: Default text anchor for relative positioning of `<text>` elements to
  center of target element.

- Fixed: Arrow connector (`class="d-arrow"`) did not display correctly on Firefox.

## [0.15.0 - 2024-12-13]

- Internal: event processing logic has been significantly refactored.

  This significantly simplifies the internals of svgdx, which should improve
  consistency and streamline improvements. The previous `ElementLike`
  trait has been replaced with an `EventGen` trait which is simpler and
  implemented by `SvgElement` as well as other types wrapping this to provide
  element-specific behaviour.

  This is a long-overdue improvement, but it does make changes to whitespace
  in rendered documents, and other minor changes may be observed.

- Internal: changed the error handling from using `anyhow` to a new `SvgdxError`
  enum. The use of `anyhow` made it too easy to treat all errors alike, which is
  unhelpful for distinguishing between recoverable (e.g. reference) and
  non-recoverable errors.

  The set of error variants is not yet fixed.

- Added: `<svg>` elements with the `xmlns` attribute may be used anywhere in a
  document to suppress `svgdx` processing the content of that element, passing
  it through unaltered.

- Fixed: bounding box calculation for `<reuse>` elements targeting a compound
  element such as `<g>` where an offset transform (e.g. `x`, `y` attributes on
  the `reuse` element) is present. Similarly fixed `<use>` elements referencing
  `<symbol>` elements.

- Fixed: avoid infinite recursion on use/reuse circular references; a new
  `depth-limit` config value (default 100).

- Fixed: specifying relative sizes for circles and ellipses, e.g. `r="#abc 25%"`

- Editor & `svgdx-server` changes:

  - implemented rate-limiting on calls to `/transform` endpoint, and added
    `defer` attribute to various JavaScript file loading. These improve behaviour
    on high-latency connections to an svgdx-server instance.
  - source line highlighting now reflects the first line of an element, not
    the last.
  - the 'Text Output' view no longer includes the source-line metadata, which
    cluttered the generated SVG.

## [0.14.0 - 2024-11-18]

- Improved text positioning, useful when needing multiple text labels inside or
  around another shape (though it's still only possible to provide a single label
  via the `text` attribute within a single element)

  * `<text>` elements positioned relative to another element using the `xy` attribute
    gain automatic `text-loc` anchoring depending on the relative position.
    For example if positioned above (e.g. `xy="#id:V"`) an element, the anchor
    will be bottom-center of the text.
  * `text-loc` attributes can now use edgespec (e.g. "b:30%")
  * New `d-text-inside` and `d-text-outside` classes can override the default
    text placement of 'inside' basic shapes / 'outside' lines & points.

- Changed: renamed `text-inset` attribute to `text-offset`, since this is equally
  usable outside shapes as inside them. Any use of this attribute will need to be
  renamed in your svgdx documents.

- Added `use-local-styles` CLI and `<config>` element option. If active, this adds a
  random `id` attribute to the top-level `<svg>` output element, and uses that with
  CSS nesting to keep all styles tied to the immediate document. This is useful when
  embedding svgdx output documents in another e.g. HTML document (as done by
  [mdbook-svgdx](https://crates.io/crates/mdbook-svgdx), for example), but may not
  function correctly in non-browser applications.

- svgdx-server / editor: Added a Content-Security-Policy for improved web-security.

## [0.13.1 - 2024-11-01]

- Added `font-size` and `font-family` config options, available in the `<config>` element
  or via command line options; defaults remain 3.0 and sans-serif respectively, but now
  text classes such as `d-text-smaller` can scale together based on the base font size.

- svgdx-server: vendored the CodeMirror5 editor source used for the online editor.
  This allows totally local use of svgdx-server without relying on any internet
  connectivity.

## [0.13.0 - 2024-10-03]

- Improved / changed: better positioning support for `<use>` and `<reuse>` elements.
  This includes use as relative positioning targets, as well as support for the SVG
  standard `x` and `y` attributes to translate where an instantiated element will be
  placed. NOTE: this implies that `x` and `y` (as well as `translate`) are acted on
  at the `<reuse>` element level, and do not become 'attribute variables' available
  in the context of the target.

- Fixed: `<symbol>` as a `<reuse>` target no longer generates invalid SVG output.
  (The `<symbol>` is translated to `<g>` element in the instantiated output, as the
  SVG spec states is the *behaviour* when a `<use>` of a `<symbol>` element occurs)

- Added: improved support for the `transform` attribute, for example generated `<text>`
  elements now inherit any `transform` from the source element, and *basic* transform
  changes in position / size (particular `translate` and `scale`) are honoured when
  constructing a bounding box. KNOWN ISSUES: interactions between `transform`,
  `<reuse>`, and `<g>` elements have various failing edge cases which will be addressed
  in a future release.

- Added: `<point>` element, used to define positions which may be referred to in
  refspec values, but (unlike a zero-radius circle) do not appear in the output.
  See the [ninedots.xml](examples/ninedots.xml) example.

- Changed: variable substitution now occurs *prior* to evaluting numeric expressions.
  Previously an expression such as `{{count($e, $e, $e)}}` with `e=""` would evaluate
  to `0`, as each variable lookup would be done as part of the expression evaluation,
  and empty values would be implicitly filtered out. Following this change, the variable
  lookup happens first, resulting in `count(,,)`, which will fail to parse and be the
  output. The advantage to early substitution is being able to do `{{$target.h / 4}}`
  (see [examples/skyline.xml](examples/skyline.xml)) with (e.g.) `target="#abc"`.
  Without early substitution, tokenisation will fail trying to parse `$target.h`.

- Changed: variables are now locally scoped to the nesting level they are defined in.
  Previously a `<var n="2"/>` inside a `<g>` element would set it permanently for the
  rest of the document (until later overridden, either by an attribute or another `var`
  element). Now variable definitions cease once their 'scope' finishes, aligning `var`
  definitions and attribute values.

- Change (minor): order of generated position attributes improved so `width` / `height`
  aren't separated by any `rx` on rectangles.

- Added: public `VERSION` constant for the library version.

- Changed / Added: if `width` or `height` are provided in the SVG root element,
  a `viewBox` will still be generated if missing. In addition, if only one of `width`
  or `height` are provided, the other will be generated automatically based on the
  aspect ratio of the calculated viewBox, including any units. For example on a 4:3
  aspect ratio diagram (after adding the border), if `<svg width="10cm">` is given,
  an additional `height="7.5cm"` attribute will be generated.

- Added: new fill pattern classes `d-grid`, `d-grid5`, `d-grid10`. These render an
  axis-aligned grid of fine lines in the themes default stroke colour at the appropriate
  scale (1, 5, or 10 user-coordinate units), and can be useful when building a diagram.
  See [examples/grid.xml](examples/grid.xml).

## [0.12.0 - 2024-09-05]

- Added: `if` element, used to selectively include a fragment. Previously this could
  be emulated with a `while` loop, but now the simpler `<if test="condition">` element
  is available.

- Added: scalarspec support in `d` and `points` attributes (for path/polyline/polygon
  elements). This is useful for `<path>` fragments such as `h #abc.w` to draw a horizontal
  line from the current position for a distance corresponding to the width of `#abc`.

- Added: initial support for themes. These are an extension of auto-styles, where
  some styles are parameterized by a theme. Current themes are `fine`, `bold`,
  `glass`, `light`, and `dark`, as well as the `default` theme which acts as previous.
  There are minor changes to the generated style rules even for the `default` theme,
  but these shouldn't change the visual display of rendered SVG images.
  One minor change is that the `background` setting now defaults to `default` rather
  than `none`; the new setting applies the theme default. This allows the value `none`
  to be explicitly provided to generate a transparent background when this is *not* the
  theme default.

- Added: new auto-styles `d-hatch`, `d-crosshatch`, and `d-stipple` for various patterned
  fill effects. NOTE: this overrides any specified colour fill (setting the theme default).

- Added: initial support for `x` / `y` / `transform` attributes on `<reuse>` elements
  aligning with the [equivalent SVG attributes on `<use>` elements](https://www.w3.org/TR/SVG11/struct.html#UseElement).
  This is only partial support; while they are carried through to a new `transform`
  attribute, bounding box evaluation doesn't (yet) take this into account.

- Added: `in(x, a1, a2, ...)` function, which returns if x is in the given list.

- Change (minor): Comments generated from `_` / `__` attributes are now surrounded
  by whitespace for readability.

- Change: The PRNG used for the `random()` and `randint()` functions is now `Pcg32`
  rather than `SmallRng`. This change ensures repeatability between native and WASM
  builds; `SmallRng` has a (documented) limitation that it is not portable.
  Any document using these functions will be change, but will become consistent between
  the WASM-based [svgdx editor](https://svgdx.net) and the command line `svgdx` binary.

## [0.11.1 - 2024-08-16]

- Added: a touch of animation comes to svgdx with the new `d-flow` auto-styles.
  Typically used on lines to indicate directionality without relying on arrows,
  this animates the stroke-dashoffset parameter where the SVG viewer supports CSS3
  animations. Suffixes `-slower`, `-slow`, `-fast` and `-faster` may be appended
  to `d-flow` to change the speed of 'flow', and the direction can be reversed
  with `d-flow-rev`. These styles automatically add a `d-dash` type appearance,
  but can also be combined with `d-dot`.

- Enhanced: `d-arrow` / `d-biarrow` styling is changed to show filled arrowheads,
  which butt-up against shapes at exactly the end of the line. This may change
  again in the future, as it's a compromise between simplicity and presentation.

- Enhanced: stroke-width auto-styles `d-thin` and `d-thick` are now joined by `d-thinner`
  and `d-thicker`. Each increment is by a factor of 2.

- Added: `var-limit` config variable (can be set by `--var-limit N` with CLI or the
  `<config var-limit="N"/>` element within a document). This defaults to 1024, and is
  the maximum length of a value assigned to a variable. This is intended to identify
  the an incorrect 'string expansion' rather than expression evaluation, e.g. when
  `<var thing="$thing + 1"/>` is used rather than `<var thing="{{$thing + 1}}"/>`.

- Changed: `loop-limit` (and the new `var-limit`) are 'maximum valid' values, rather
  than the point at which things break (as previously).

- svgdx-editor: limited zoom speed from `wheel` events to make scrolling with a
  trackpad more controllable (and less CPU intensive!)

## [0.11.0 - 2024-08-07]

- svgdx-editor: now uses WASM for browser-local transforms, and server transforms when
  using svgdx-server. See [svgdx.net](https://svgdx.net) hosted using GitHub Pages.

- More refactoring to support bounding boxes for `<g>` elements, with the following changes:
  * Added: `<g>` groups now have bounding boxes and can be used as the target of a relspec.
  * Removed: the `repeat` attribute is no more; use `<loop>` instead. While `<loop>` is
    more verbose, it more cleanly separates 'control' elements from 'graphic' elements.
  * Internal: use of traits to define element behaviour, allowing different classes of
    element to behave differently.

- Fixed: `loop-limit` can be set in `<config>` elements, as documented.

- Fixed: variable expansion now works in `class` attributes.

## [0.10.0 - 2024-07-04]

- Substantial refactor of positioning / layout logic with following key changes:
  * Changed: 'relspec deltas' such as the '1 2' in 'xy="#abc@t 1 2"' indicate an
    offsets from the point given by the relspec as an (x, y) pair. Previously if
    only a single value was given, it would be the 'dx' value with 'dy' implicitly
    set to zero. From this release a single value is *duplicated* across both dx
    and dy, equivalent to a single entry in a `dxy` attribute.
  * Added: constraint-based positioning. The x1/x2/y1/y2/cx/cy/width/height attributes
    (and compound combinations, e.g. xy1, cxy) may be provided for *any* basic shape,
    providing the position is sufficiently tied down. For example, a circle may be
    specified by the combination `y2`, `x1` and `r`, or the combination `cx`,  `y1`,
    and `x2`.
  * Changed: attribute ordering is no longer preserved. Given the additional number
    of positional source attributes and transformations around these, a different
    approach of forcing selected generated attributes to appear in a specific order
    is used - for example in a rectangle the generated `x` / `y` / `width` / `height`
    attributes will always appear in that order, and any `id` attribute will always
    be the first attribute of an output element.

- Added: various functions making use of expression lists:
  * Rectangular / polar conversion functions `r2p(x, y)` and `p2r(r, theta)`.
  * Vector-arithmetic functions `addv(a0, a1, ...aN, b0, b1, ...bN)`,
    `subv(a0, a1, ...aN, b0, b1, ...bN)` and `scalev(s, a0, a1, ...aN)`,
    where `addv` and `subv` return the element-wise sum / difference of each list,
    and `scalev` multiplies each element by the first `s` argument.
  * List processing functions: `head()` (return first entry),
    `tail()` (remainder after removing head), `empty()` (return 1 if the
    given list is empty, else 0), and `count()` (cardinality of list) -
    each taking a (possibly empty) list, and `select(n, a0, a1, ...aN)`
    to retrieve the nth item in the list.

- Added: support for `loop-var`, `start` and `step` attributes in `while` and
  `until` loop types (in addition to `count`).

- Added: the `repeat` element is now available within `<reuse>` targets and
  supports expressions rather than only hardcoded integer counts.

## [0.9.3 - 2024-06-14]

- Change (minor): `style` attributes are no longer copied into any `<text>` elements generated
  by `text` attributes, as conflicts between 'outer' and 'text' element styles are common.
  A new `text-style` attribute is provided which becomes the `style` attribute of any
  generated text element.

- Added: Support for multiple (comma-separated) expressions in a single `{{...}}`
  block. This also provides the concept of an 'expression list', which is a natural
  fit for variadic functions. The `min()` and `max()` functions can now take any
  number of arguments, as can the new variadic `sum()`, `product()` and `mean()` functions.
  Note all these functions require at least one argument.

- Fixed: don't attempt to derive bounding box for elements (including root `<svg>`)
  where size / position attributes have units or %age values - e.g. `x="5%" width="10cm"`
  etc. Previously this prevented processing many SVG examples using units or percentages.

- svgdx-editor: Added 'Copy PNG' button(s) which copy a PNG image to clipboard in the
  selected resolution.

## [0.9.2 - 2024-05-28]

- Fixed (regression in 0.9.0): overriding compound attribute derived positions
  caused position deltas (dx/dy/dw/dh) to stop working correctly in some cases.

- Change (minor): `text-dx`/`text-dy`/`text-dxy` are now applied *after* any text
  insetting, rather than *instead* of insetting. This means `text-dxy="0"` is now
  a no-op.

- Added: Support a `text-inset` attribute for varying the inset value (i.e. how
  much non-centered text is 'pulled in' from a corner or edge). Previously this
  was hard-coded to 1.

## [0.9.1 - 2024-05-27]

- Added: support for vertical text; add the `d-text-vertical` class to an element
  to cause text to be rendered vertically. The same integration with basic shapes
  and use of the `text-loc` attribute if available for vertical text.

- Added: improve loop ergonomics with support for `loop-var`, `start` and `step`
  attributes on `count` loops. By default the variable assigned by `loop-var` will
  take the values `0`..`count-1`, incrementing each iteration. `start` and `step`
  allow overriding the initial value and increment. Previously using a loop counter
  as a variable value required a combination of two `<var>` elements (for initialisation
  and increment) and a `while` loop.

- Fixed: attributes derived from compound attribute expansion are lower priority
  than equivalent target attributes, i.e. an `xy` attribute should not overwrite
  an existing `y` attribute.

- Fixed: various error handling improvements

## [0.9.0 - 2024-05-13]

- Added: support for `<loop>` element, with `count`, `while` and `until` attributes
  defining multiple iterations of the contained elements. See the [loop element](docs/elements.md#loop)
  docs for more information.

- Added: element `id` attributes are now evaluated as with other attributes,
  allowing id value to include variable values; useful to keep `id` unique
  inside loop bodies.

- Added: Check for circular references in expressions to avoid stack overflow.

- Fixed: positioning relative to a connector (`<line>` with `start`/`end` attributes).

- Fixed: `dx` / `dy` attributes were being incorrectly stripped from `feOffset` elements.

- svgdx-editor enhancements: Increased number of tabs from 5 to 10; added 'text output'
  view with toggle button; minor styling improvements.

## [0.8.0 - 2024-04-27]

- Changed: various auto-styles relating to text, including the ability to specify
  text colour, size, style, and a change to prevent `d-softshadow` / `d-hardshadow`
  being applied to rendered text.
  A new `d-thick` style has been added as a complement to `d-thin`.
  See [auto_styles.md](docs/auto_styles.md) for more information.

- Changed: attribute-based variable lookup now starts at the parent (rather than
  the current) element. Note that when `reuse` is in effect, the source `reuse`
  element itself is considered the immediate parent of target element.

- Added: `<reuse>` element now supports `<g>` targets allowing more complex shapes
  to be reused. Note that (currently) there are still limitations around positioning;
  `<g>` doesn't yet have it's own bounding box, and elements within a reused group
  element do not support the multiple passes required for forward references.

- minor svgdx-editor enhancements including timestamp in download filenames

## [0.7.1 - 2024-04-07]

- svgdx-editor enhancements, including both horizontal and vertical editor layouts,
  copy as data URI, highlight source lines on element hover, and various bug fixes.

## [0.7.0 - 2024-04-01]

- Changed: Split out features for `cli` and `server` (support for the new
  `svgdx-server`). Both these are default features, but if only the library
  is needed a no-default-features build will be smaller / faster to compile.

- Added: 'svgdx-server' - a simple web-based frontend for editing and viewing
  svgdx documents. Run with `cargo run --bin svgdx-server`. I'm not sure what
  direction this will go in - for me it's a workflow improvement over the
  `--watch` + [Gapplin](https://gapplin.wolfrosch.com) (or similar) I'd been
  using previously.

- Added: Support `wh` and `xy` attributes for `<use>`, `<image>`, `<svg>` and
  `<foreignObject>` elements.

- Changed / Added: `text-pre` attribute which if present (with any value) converts
  spaces in text to non-breaking spaces, to defeat the XML whitespace processing
  and allow space-preserving text to be displayed. Previously leading spaces were
  converted in this way, but not internal space.

## [0.6.0 - 2024-03-10]

- Changed: relspec positioning modified to be more consistent: the referenced element
  must now be given explicitly as `^` or `#id`, rather than the previous implicit
  'previous element' default. Direction-based relative positioning ('dirspec') now
  always requires a ':' separator. See [layout.md](docs/layout.md) for details of the
  new relspec definition.

- Changed: `margin` now takes up to 4 entries, analogous with CSS margin and padding
  values. Previously it took either one or two entries, and with two entries the
  first was an 'x' margin and the second a 'y' margin. Now the [CSS approach](https://developer.mozilla.org/en-US/docs/Web/CSS/Shorthand_properties#margin_and_padding_properties)
  is used with support for separate TRBL margins.

- Added: `inside` attribute, analogous to `surround` but as the intersection of
  bounding boxes rather than their union.

- Added: support for providing text content for graphical elements using XML text or CData
  in addition to the `text` attribute. This makes pre-formatted text much easier to include.

- Changed: leading whitespace in text content of elements is now replaced with non-breaking
  spaces, to allow arbitrary indenting.

- Added: Relational and logical functions. Function notation (e.g. `lt(a,b)` rather than
  `a < b`) is chosen to avoid the need to use XML entities ('`&lt;`' etc). It also avoids
  the need for more levels of precedence, though that's not a good reason on its own.
  Note logical values adhere to C-language values; non-zero evaluates to true, and true/false
  will be indicated by the numeric values 1.0 & 0.0 respectively.

## [0.5.0 - 2024-02-11]

- Changed: Removed the custom elements `<tbox>`, `<person>` and `<pipeline>`,
  since these are now better implemented with `<specs>` and `<reuse>`.
  The `person` and `pipeline` examples have been updated to use the new `<reuse>`
  templating approach.

- Added: Support for `<specs>` and `<reuse>` custom elements to allow templating of
  elements in the rendered document.

- Changed: Variable lookups now first reference attributes of the input element
  (provided this is not a `<vars>` element) before checking the global namespace.
  This is particularly useful in conjunction with the `<reuse>` element to provide
  'custom element'-like behaviour.

- Added: a bunch of builtin functions for expressions, including basics such as min,
  max, and abs, trigonometric functions, and random number generation. See the
  [function documentation](docs/expressions.md#built-in-functions) for a full list.

- Added: a `--seed` command-line argument (and `<config seed="...">` setting) to
  initialise the random number generation for the `random()` and `randint()` functions.

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
