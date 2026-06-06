# Delta 8 - Reuse, defaults, and custom elements

> `svgdx` fragments may be instantiated - with variation - later in a document

## Overview

This page discusses how content in your svgdx document may define templates which may later be instantiated.

It starts by discussing SVG's `<use>` element, provided as part of standard SVG, before moving on to discuss the `<reuse>` element provided by svgdx allowing parameterised re-use, syntax sugar allowing custom elements to be defined, and the `<defaults>` element allowing a level of attribute inheritance.

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

### The `<specs>` container element

With `<reuse>` duplicating the entire element into the rendered document, there is no benefit to keeping the source definition in the rendered document: a new `svgdx` element named `<specs>` is introduced that acts equivalently to SVG's `<defs>` element but doesn't appear in the rendered document.

One of the key ways structural changes can be made is through the use of _context variables_.
From the perspective of the referenced element, these are normal variables, used with the `$name` syntax as part of attribute values. The _source_ of these variables is not a `<vars>` or `<loop>` element, but additional attributes on the `<reuse>` element itself.

When using injected context variables, the template may not be valid at the point it appears in the input document (i.e. the variables might not be defined at the point the template appears).
In a `<defs>` container `svgdx` would still attempt (and fail) to evaluate variables, while elements within a `<specs>` element are deliberately not evaluated in any way until referenced from a `<reuse>` element.

```xml-svgdx
<svg>
 <specs>
  <symbol id="a">
   <rect wh="15" class="d-fill-$colour"/>
   <circle r="5" cxy="^" text="$colour"/>
  </symbol>
 </specs>
<reuse href="#a" colour="blue"/>
<reuse href="#a" x="20" colour="red"/>
</svg>
```

## Custom elements

Custom elements may be defined within an svgdx input document using reuse semantics. This feature is implemented as syntax sugar over `specs` and `reuse`.

Rather than using `<specs>` purely as a container for elements referenced via `<reuse>`, a `<specs>` element with an `element` attribute defines a custom element.

```xml
<specs>
  <symbol id="name">
    ...
  </symbol>
</specs>
<reuse href="#name" ...>
```

can be rewritten as the more semantic

```xml
<specs element="name">
  ...
</specs>
<name .../>
```

Note that not all values of `name` are valid as custom element names - at least the following are reserved by svgdx:

 * `config`
 * `reuse`
 * `specs`
 * `defaults`
 * `var`
 * `if`
 * `loop`
 * `for`

> NOTE: Avoid defining custom elements that conflict with standard SVG elements.
> The list of reserved elements may expand in future.

```xml-svgdx
<svg>
  <specs element="document">
    <!--
      name: document
      variables: width, height, text
    -->
    <var fold="{{min($width, $height) / 3}}"/>
    <path d="M 0 0 H {{$width - $fold}} L $width $fold V $height H 0 Z
             M {{$width - $fold}} 0 V $fold H $width" style="fill: whitesmoke"/>
    <text xy="^@c" text="$text"/>
  </specs>
  <document xy="0" width="15" height="20" text="ABC"/>
  <document xy="^|h 10" width="20" height="10" text="DOC"/>
</svg>
```

## The `<defaults>` element

The `<defaults>` element provided by svgdx is normally used as a _container_ element - surrounding another group of elements which act as 'blueprints' for setting the attributes of matching elements within that scope.

Suppose we want all rectangles to have rounded corners, and a default size of 30x10. We can encode these as default attributes for all rectangles:

```xml-svgdx
<svg>
  <defaults>
    <rect rx="2" wh="30 10"/>
  </defaults>
  <rect text="hello!"/>
  <rect xy="^|v 5" rx="5" text="rounder"/>
  <rect xy="^|v 5" height="15" text="height\noverride!"/>
</svg>
```

There are several concepts to be aware of when using the `defaults` element:

  * Defaults are **scoped** to the current nesting level and below. For each attribute, a lookup for the default value to use starts at the inner-most nesting level and bubbles up to the root element until a match is found or no default is specified.
  * Defaults apply to **matching** elements: the simplest case is the element name being the same (as in the earlier example using `rect`), but the `<_ .../>` element (i.e. element '_') may be used to match all element types, with the `match` attribute in elements applying further restrictions on either class or element type. The syntax roughly matches very basic CSS selectors: comma separated 'alternate' matching, with `element.class1.class2` or `.class3` type selectors in each of the comma-separated parts. Only element type and class-based matching are implemented.
  * Certain attributes are **augmented** rather than **set if absent**. In the example above, the `rx` and `wh` attributes are set on the target when not already present, but for the following attributes any defaults are _appended_ to existing values:
    * `class`
    * `style`
    * `text-style`
    * `transform`
    > NOTE: this doesn't always work as expected, as no concept of 'related' classes or styles exists. Setting a default colour through a default class and later attempting to override it with a locally defined colour class will result in both colour classes or styles defined on the target.
  * While `<defaults>` is typically used as a container, any attributes defined directly on this element are equivalent to those on a contained `<_ .../>` element, so is also useful asas an **empty element**. `<defaults .../>` is equivalent to `<defaults><_ .../></defaults>`, i.e. it applies to every element subject to any provided `match` attribute.

The following fuller example shows these in practice.

```xml-svgdx
<svg>
  <defaults rx="1" xy="^|h 2" wh="5"/>
  <rect xy="0"/>
  <g>
    <defaults wh="12" class="d-thick">
      <rect class="d-text-italic"/>
      <circle class="d-dot"/>
      <_ match=".error" class="d-fill-red"/>
      <_ match=".warn" class="d-fill-orange"/>
      <_ match=".ok" class="d-fill-green"/>
    </defaults>
    <rect text="stop" class="error"/>
    <circle text="ready" class="warn"/>
    <rect text="Go!" width="20" rx="3" class="ok"/>
  </g>
  <circle />
</svg>
```

---

[^1]: With some caveats; in particular a referenced `<symbol>` element is converted into an equivalent `<g>` element.
