# Delta 8 - Reuse

## The `<use>` element

SVG supports re-use of elements through the `<use>` element. Its `href` attribute references another element, which is instanced at the point of the `<use>` element. Instance position can be provided through `x` and `y` attributes; in svgdx the standard universal positioning and compound attributes can be provided to position instances relative to one-another.

Typically the `href` target of a `<use>` element is not a visual part of the document, but referenced from an element within the `<defs>` container - which contains 'definitions' which aren't directly rendered. In addition, the `<symbol>` element acts in the same way as the `<g>` element, but is not itself rendered. Typically `<symbol>` elements will also be inside a `<defs>` element, though this does not affect the document rendering.

Typical use of `<defs>`, `<symbol>` and `<use>` will include several elements all defined at the origin, and then instanced at particular positions through the `<use>` element.

The following example shows `<use>` elements in action.

```xml-svgdx
<svg>
 <defs>
  <symbol id="a">
   <rect wh="10" class="d-fill-grey"/>
   <circle r="3" cxy="^" class="d-fill-red"/>
  </symbol>
 </defs>
<use href="#a"/>
<use href="#a" xy="^|h 3"/>
<use href="#a" xy="^|v 3"/>
<use href="#a" xy="^|H 3"/>
</svg>
```

A limitation of the `<use>` element is that each instance is an exact copy of the original; while some styles and transforms can be applied to the element, the overall structure is identical.

Where *similarity* rather than exact instancing is the order of the day, the `svgdx` extension element `<reuse>` is available.

## The `<reuse>` element

The `<reuse>` element is modelled on `<use>`, and in most cases any `<use>` element in an svgdx document could be changed to `<reuse>` without a change in the document appearance.

* `<use>` is a standard SVG element - it is efficient in document size. It is up to the client application (e.g. image viewer, browser etc) to render the `<use>` element appropriately.
* `<reuse>` _replicates_ the referenced element as-is at the instance site[^1]. This is usually less efficient, especially if the referenced element is complex, but allows structural changes to be made.

```xml-svgdx
<svg>
 <defs>
  <symbol id="a">
   <rect wh="10" class="d-fill-grey"/>
   <circle r="3" cxy="^" class="d-fill-red"/>
  </symbol>
 </defs>
<reuse href="#a"/>
<reuse href="#a" xy="^|h 3"/>
<reuse href="#a" xy="^|v 3"/>
<reuse href="#a" xy="^|H 3"/>
</svg>
```

One of the key ways structural changes can be made is through the use of _context variables_.
From the perspective of the referenced element, these are normal variables, used with the `$name` syntax as part of attribute values. The _source_ of these variables is not a `<vars>` or `<loop>` element, but additional attributes on the `<reuse>` element itself.

```xml-svgdx
<svg>
 <defs>
  <symbol id="a">
   <rect wh="15" class="d-fill-$colour"/>
   <circle r="5" cxy="^" text="$colour"/>
  </symbol>
 </defs>
<reuse href="#a" colour="blue"/>
<reuse href="#a" x="20" colour="red"/>
</svg>
```

---

[^1]: With some caveats; in particular a referenced `<symbol>` element is converted into an equivalent `<g>` element.
