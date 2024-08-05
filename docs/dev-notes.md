# Dev Notes

This living document covers *active* design thinking / questions.

Once the design is perfected (lol!) there shouldn't be anything left here.

## Design gotchas

### Per-element pipeline

A clean design here would be something like:

1. Derive SvgElement from event(s), including any tail or content
2. Evaluate attributes (e.g. expressions, variable lookup)
3. Resolve positioning, setting updated attributes as required
4. Render events required for the modified element(s), including any relevant whitespace

This probably works effectively for a simple input document.
It gets tricky where things like 'evaluate attributes' has side-effects,
such as iterating a PRNG, or setting variables (especially things such
as `k="{{$k + 1}}"`), which are not idempotent.

This requires care to ensure these operations only happen once during
the pipeline (while restoring variable state might be ok for the 'increment'
side effect, un-getting a random number from a generator isn't something
I want to deal with).

There are also cases where info is only known at a later point (e.g.
forward references for element layout, or evaluating bounding boxes of
container elements). Sometimes we need to back out of processing in order
to re-try later, but this typically happens during 'resolve positioning'
step, and if the 'evaluate attribute' step has already happened, side-effects
to the context have already occurred and may not be un-doable.

One approach would be to do a topological sort on references early on,
avoiding the need to 'abort and retry' on reference errors, but that
fails if we have e.g. `id="thing-$k"` or `xy="#{thing-$k}:h"`, where
the references depend on 'current' document state. (I'm keen to keep the
ability to have non-static ids.)

### Error handling

The current error handling of svgdx isn't great; end-user experience is OK,
with unresolvable elements output with line number info, but internally it's
a bit of a mess.

The current state effectively uses `Result` objects and `?` propagation to
emulate exceptions, and post-hoc checks of 'did it go better this time' to
determine whether progress has been made or the processing has stalled.

`anyhow` has made this easy, but I'm not sure it's good; the code really needs
to distinguish between 'retryable' errors (e.g. reference lookup to something
which might be a forward reference) and unrecoverable errors. Other than reference
errors propagated to enable forward references, many error conditions just
leave attribute values (for example) unchanged; having `<circle r="thing"/>`
should probably be a hard error rather than being left as-is; it's more likely
there's a missing '$' in there somewhere, and reporting this as an error
would be useful.

Alternatively having warnings when e.g. `strp` fails might be helpful, though
the current system only returns errors or the document. Perhaps metadata
analogous to the `data-source-line` - e.g. `data-warning` - would allow the
client program (using `config.add_metadata`) to extract the info without
changing to return the document and some out-of-band data?

Tidying up the error handling would also improve the UX; it wouldn't just list
unresolvable elements, but *why* a particular element could not be resolved.

Therefore should probably investigate `thiserror` or similar, either to augment
or replace `anyhow`.

### Regex replacement

During earlier WASM investigations using `twiggy` showed that well over 50% of
the code size was due to `regex` handling. Even after moving to `regex-lite` it's
still a significant proportion of the code size.

Should investigate moving to `nom` to both centralize parsing and see if it has
code size / perf improvements (or maybe just hand-code something - there's nothing
particularly tricky AFAICT).

## Open questions

### '%' behaviour in expressions

Should the `%` operator in expressions actually be 'modulus' rather than
remainder (i.e. always positive; implementation would use `T::rem_euclid()`)?
Note Python uses modulus (i.e. `-3 % 10 == 7` in Python), whereas Rust uses
remainder (i.e. `-3 % 10 == -3` in Rust).
Given a set of things with some index, and always wanting to index into them,
having `-3 % 10 === -3` isn't very helpful (vs the 'expected' value 7).

Therefore should probably make breaking change and switch to modulus.

### Round-tripping / preserving SVG document structure

Early on the goal for `svgdx` was that any SVG document not using svgdx extensions
would be untouched by processing. Structures such as `AttrMap` and `ClassList`
carefully maintain input order. However this increases complexity in various places
(e.g. handling text between elements, rather than as element content).

Relaxing the 'no-op for SVG' requirement, and instead ensuring that any *output*
of svgdx processing would be untouched (i.e. round-tripping for a subset of SVG,
rather than effectively all possible XML) would make things easier and more
consistent.

Some middle ground is still possible here, e.g. if an element starts on a new line
in the input, do the same in the output. But preserving arbitrary XML 'tail' text
(rather than recreating it based on line number and indent values) may not be necessary.

Early in svgdx development I was very focussed on whether the output *text* looked
'right', but I'm now using svgdx-editor a lot more during development, and care
more about whether the output document *rendered as SVG* looks 'right'.

Therefore should probably give up on trying to process arbitrary Text / CDATA events
and immediately connect them as element content, and recreate whitespace based on
indent values. (Note 'recreating' whitespace is already necessary when additional
elements are generated, such as from `text` attributes on graphics elements.)

### Self-consistent, or consistent with SVG?

Consider `<use>`. It takes `x` and `y` attributes, which are (semantically) translated
by an SVG user agent into `transform="translate(x, y)"` on the instanced target.

For consistency with this, `<reuse>` should probably do the same thing. But internal
consistency and uniform positioning (i.e. that any sufficient constraint on x/y/x2/y2/...
will position elements) would imply that `x`/`y` denote the top-left of the target
bounding-box... In some cases this is the same, but if a circle defined with `cxy="0"`
is `<use>`d then the bounding box will be `r` to the upper left of the given `x`/`y`
attribute pair...

Should probably prioritise consistency with SVG, and have `<reuse>` be as similar as
possible to `<use>`, so it's main benefit is in templating where context-dependent
expressions make a difference. Downside: `x` / `y` are special cased, where other
attributes are effectively local variables for the re-used target.
