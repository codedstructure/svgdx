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
fails if we have e.g. `id="thing-$k"` or `xy="#{thing-$k}|h"`, where
the references depend on 'current' document state. (I'm keen to keep the
ability to have non-static ids.)

### Bounding box modifiers

There are at least a couple of 'modifiers' on bounding boxes: `transform`
attributes, and `clip-path` references. Both of these can change the bounding
box of an element.

Ideally we'd only compute the bounding box of an element once, but currently
it is evaluated potentially every time it is referenced. Part of this is that
context can matter; in the case of a `<reuse>` element, variables might impact
the final bounding box differently on every instantiation.

There are also two paths involving bbox computation: one is when an element
is referenced as an elref from another element, and the other is in computing
the total bbox of the containing element (possibly the root SVG element). The
latter doesn't go via the `get_element_bbox()` function, but simply accumulates
bounding boxes of elements at the same level as they are computed.

Simplifying and unifying all this (as well as reducing the number of ad-hoc
`update_element()` calls) would improve the maintainability of svgdx.

### Positioning approach

Elements exist at one of three positioning levels:

* Unknown / indeterminate
* Size known
* Full bounding box known

Keeping the second two separate is important since some elements don't have a
position (e.g. a symbol in a defs group, or a group in a specs block which will
be referenced from a <reuse>). This largely applies to reuse (i.e. `<use>` or
`<reuse>`) where multiple instances of a target may exist at different locations.

However knowing the size is useful even if the full location is not known:

* TODO - explain why, if there is a good reason...

#### Positioning Principles

Where reasonable, shapes should be located with their own attributes, e.g. `x`
and `y` in the case of a `<rect>`.

For `<g>` elements, a `transform` should be used instead - this avoids having
to apply location to all elements within the (possibly nested) group.

With `<polyline>` / `<polygon>` elements, either approach (translate / update
each `points` coordinate) is reasonable, but for `<path>` elements it becomes
more tricky, since there is a mix of relative and absolute values. Just
translating the whole lot is quite attractive. (If the whole `d` attribute used
relative commands, then a simple update to the initial `m` / `M` command would
be nice, but that's not a reasonable constraint).

So, to keep things simple:

* use/rect/circle/ellipse/text - update the x/y (or equivalent) attributes.
* polyline/polygon/path/g - use a `transform="translate(x,y)"` attribute.

If we know the *size* of the item and a sufficient set of position attributes,
this lets us place any of these objects. If we *don't* know the size, we could
still place some objects given the top-left x/y point, and others (circle/ellipse)
with their center point, but feels arbitrary.
In addition we'd have to give up the useful `relpos` based positioning, which
requires knowledge of both the referenced object's full bbox as well as the
current object's size.

We have a `Position` class which captures knowledge of various element dimensions,
and once sufficient this should be able to write out the attributes of a target
element. (Including e.g. transform attributes as appropriate).

A key use-case of this is to handle reuse, where a copy of an instanced element
may be applied in many different locations. It also allows a universal positioning
approach, where all elements can be positioned using e.g. 'x2, cy' parameters -
assuming that the element's size is known.

So, for positioning:

* general attribute evaluation within current context
* expand compound size attributes (wh, dwh)
* derive size (width, height, dw, dh - including appropriate element lookup)
* if `xy` exists and is a relspec:
  * require target element has a full bbox, else RefErr
  * given size, set the anchor point as required considering gap as dx/dy.
* else:
  * expand compound position attributes (xy, cxy, xy1, xy2, dxy, etc)
  * derive position (including element lookup)
* if position is sufficient, apply element-appropriate attributes to the target (and remove any obsolete ones)

The above implies that all positioning is done via the Position struct, even for
basic cases. Is this reasonable?

Is it reasonable to disallow / disregard transform on input? => Probably yes...?

Suggests each element should include a Position object... possibly with enum
to hold it's state (e.g. final, provisional - e.g. may be expanded, etc)

#### Re-use positioning

For reuse elements, both the targeted object and the reuse element may have position
attributes. How should these be combined?

* If the target element is in a defs block etc, it may have a 'deliberate' offset, e.g. xy="5".
  In that case it might be natural to see the xy on the reuse as an offset, (i.e. dxy) to be
  added to the original.
* If the target element is already a visual thing used elsewhere, it is very unlikely to be
  at the origin anyway, and using an offset would be counter-intuitive. The 'thing itself',
  defined by its bbox, would be what should be positioned.
* Therefore, xy on the reuse should translate the bbox of the target, *overriding* any position
  it may have.

## Open questions

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

### Auto-numeric Expression Handling

In general, expressions evaluated in `{{..}}` contexts are converted to numbers,
and function calls are evaluated in place. However in some conditions (e.g. `while`
conditions on loops) the value must always be a number, so the brace pairs may be
omitted.

Should this be extended to every attribute which requires a numeric value? Or remove
the shortcut on the existing evaluations using this? Consistency doesn't have to be
absolute, but where it is lacking there needs to be a principled / documented reason
for it, which currently isn't the case.

Motivation: should `<loop count="$expr">` be changed to automatically use numeric
evaluation for `$expr`?

### Previous N reference

It's often useful to refer to the 'last-but-one' element, which isn't currently
possible without using an `id`.

Perhaps something like `^^`, with reasonable extension (e.g. no more than 10)
to earlier elements. This would allow nicer grids which need to alternate `|h`
and `|v` type relative positions.

### Auto-style class combinations

Is it better to have combination class names for auto-styles, or require multiple
classes to implement this?

e.g. is it better to have `class="d-flow-rev-fast"` or `class="d-flow d-flow-rev d-flow-fast"`,
or some combination where only one parameter (e.g. flow speed) is allowed as part of
the 'base' class name? (so `d-flow-rev` is a separate 'boolean' flag, but speed can
be included as e.g. `d-flow-faster`). If a 'simple' reverse flow is needed, this does
require `class="d-flow d-flow-rev"`, which feels verbose, as though `d-flow-rev` should
imply `d-flow`?

The first is clearly more concise, but it's not obvious which order the various aspects
should be. Maybe they could be entirely dynamic, including use of numbers in class
names? e.g. `d-text-size-12` or similar.

The existing text attributes tend to use the 'separate classes' approach, e.g.
`class="d-text-bigger d-text-mono"`

Note ergonomics should drive this rather than ease of implementation.

### Auto-style rules - inherited in nested elements?

If a class such as `d-red` is applied to a `<g>` element, should it apply to all the
contained elements (which don't override it)?

Applying to `.d-red, .d-red *` or similar would do, or even `.d-red g.d-red *`, but
will this allow override be a more local application of a class?

The following is an example of how this could work. Note that if the second rule of
each pair is `g.b-xyz *` rather than `.b-xyz *`, the specifity exceeds the bare `b-blue`
of rect 'd', so that's not feasible. With CSS, being more precise results in increasing
priority, when it would be nice if these could be independent...

```xml
<svg>
  <style>
    .b-red:not(text), .b-red *:not(text) {fill:red;}
    .b-blue:not(text), .b-blue *:not(text) {fill:blue;}
  </style>
  <rect wh="3" text="a"/>
  <rect wh="^" xy="^|v 2" class="b-red" text="b"/>
  <g class="b-red">
    <rect wh="^" xy="5 0" text="c"/>
    <rect wh="^" xy="^|v 2" class="b-blue" text="d"/>
  </g>
</svg>
```

Should probably change most of the rules to include the `.class` to be `.class, .class *`.

### Local attribute ordering.

Consider the following two use-cases of local attributes:

* **group / scope** level - here the _closer_ the definition of the variable is to the actual
  instance, the higher the priority it has.

* **reuse** scope - here the _further away_ the definition of the variable is, the higher
  priority it has. Closer definitions may act as defaults, but they can always be overridden
  by the "call site" of the instancing.

Both these cases assume that there are multiple levels of hierarchy, i.e. nested `<g>` elements
or `<reuse>` elements which refer to other `<reuse>` elements. In either case, the actually
'instantiation' place is the most important and most local, but the cases differ in where that
is relative to the actual elements which represent rendered shapes.

The group scope approach works fine as is, but the reuse scope only works 'one level deep' -
if a top-level `<reuse>` defines an attribute-variable, this will be used in the final render.

But 'multi-level' reuse doesn't currently follow this naturally, i.e. the following doesn't
work, when it seems a reasonable use-case to have 'defaults' (and potentially multiple layers
of specialisation):

```xml
<specs>
  <g id="a"><rect xy="0" wh="5" text="$t"/></g>
  <reuse id="t1" href="#a" t="1"/>
  <reuse id="t2" href="#a" t="2"/>
  <reuse id="t3" href="#a" t="3"/>
</specs>
<reuse href="#t2"/>

<!-- ideally this would render the text '2', but currently just renders '$t' -->
```

### Variable lookup in expressions

When should variable lookup happen?

It seems useful to have element references which may be parameterized by a variable,
e.g. `xy="$el|h"`, and then define `el="#blob"` elsewhere. This is catered for fine;
prior to any positioning, attributes are evaluated, and at that point it will be
replaced with `xy="#blob|h"`, which later feeds in to positioning logic.

Note this implies a further sequencing operation: 'compound' attributes such as `xy`
already have an implicit 'splitting' action, such that `xy` will be split into `x`, `y`
attribute pairs if there isn't a relative position (e.g. '|h') involved. But this is
still reasonable: first expand variables, then check for relative positioning, then
finally split (as appropriate) into different target attributes.

Where this gets more complex is when a variable defines another variable.

```xml
<var v1="1" v2="2"/>
<var select="v1"/>
<var target="$$select"/>
<text text="$target"/>
```

### Empty entries in variable lists

What should the following return?

```xml
<text text="{{count(,,,,)}}"/>
```

There are three reasonable alternatives:

* It should return 0, having first filtered out any empty entries (in `expr_list()` or
  similar).
* It should return 5; a comma implies entries on either side, so should be n+1 where n
  is the number of commas present. There should probably be an additional 'Null' variant
  of `ExprValue` to account for this.
* It is an error, and therefore is unchanged. This is the current behaviour. This aligns
  with expressions (including all current functions) taking only numeric values.

This should probably (only) be revisited once `ExprValue` is extended to take string values
(both lists and single values).

### Variable types: const and global

Variables are 'local' to the scope they are defined (through attributes on a container
element) or set (via the `<var>` element). Once any nested element closes, the variable
values set in that scope disappear / revert to any previously set values.

This is probably the expected behaviour and seems a sensible default.

However there may be cases where 'global' variables which ignore scope are useful. One
example would be to count the number of times a particular fragment is `reuse`d; this
could be done with something along the lines of `<global reuse_count="{{$reuse_count + 1}}"/>`

Whether `global` is the right element name, or it should be done through some other
convention (e.g. variables beginning with a certain character?) is an open question.

A separate use-case would be for 'constant' values, where they are frozen when first set
(and possibly generate errors if a subsequent set - at least to a different value - is
attempted).

This could be useful for 'constants' such as $PI, $SQRT2 and similar, and might be implemented
using something like `<const PI="3.1415927"/>`.
