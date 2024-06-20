# Delta 7 - Loops and Reuse

> svgdx goes beyond SVG's re-use capabilities

## Reuse in SVG

The primary mechanism in SVG for reuse is the `<use>` element, which allows duplicating an existing element (possibly a group) or instantiating a `<symbol>` from a `<defs>` block.

Where this behaviour is still appropriate, it should generally be used; svgdx isn't intended to replace SVG behaviour which already works perfectly well.

There are various enhancements which svgdx *does* provide however. One of the commonest forms of reuse is to have multiple copies of an element (group); this can be done through use of the `repeat` attribute or the `<loop>` element.

In addition, a new `<reuse>` element is provided which copies the target element(s) into place, rather than simply providing a reference to them (as `<use>` does). This allows modifications beyond basic positioning in each instance, and is more like templating.

## Repetition

### The `repeat` attribute



### The `<loop>` element


## Templating

If an element needs reusing in a new location, possibly with some `transform` applied, then `<use>` has you covered. But if some deeper change in the referenced element is required, duplication-with-change may be a better fit, and svgdx's `<reuse>` element provides this. Rather than *reference* the target element, `<reuse>` replaces itself with the target element. The power of this is that the context of rendering the replacement is augmented by the `<reuse>` element's attributes.

Suppose we are diagramming a CPU, perhaps for

### The `<specs>` element

Because `<reuse>` provides a full copy of the target element, there is no 'SVG-time' reference present, and the original 'blueprint' for the rendered elements is no longer required.

svgdx supports 'build-time only' elements through the `<spec>` element. Anything inside a `<spec>` block is omitted from the rendered SVG document - it is present only during the conversion step to be referenced as required.
`<reuse>` is the primary use-case for this, but anything which is required for processing but not wanted in the output may be placed in this section.
