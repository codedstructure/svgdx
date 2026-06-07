# Elements

## SVG Elements

The primary element type in the source document is likely to be SVG element types, though many support additional [attributes](attributes) to provide easier specification or other functionality.

See the [SVG spec](https://www.w3.org/TR/SVG11/) for further details on these elements.

## Custom Elements

### `config`

This element allows a document to provide it's own configuration settings which would otherwise be provided on the command line.
Normally this element should be provided at the start of a document.

Values are given as `key="value"` attribute pairs; multiple key-value config pairs can be provided in a single `<config>` element.

The following configuration settings can be applied using this element. These correspond to equivalent command line options.

| Name | Type | Example | Notes |
| --- | --- | --- | --- |
| debug | bool | `debug="true"` |
| background | [colour name](https://www.w3.org/TR/SVG11/types.html#ColorKeywords) | `background="lightgrey"` |
| scale | float | `scale="2.5"` |
| border | integer | `border="20"` |
| font-size | float | `font-size="5"` |
| font-family | string | `font-family="Ubuntu Mono"` |
| loop-limit | integer | `loop-limit="9999"` |
| var-limit | integer | `var-limit="4096"` |
| depth-limit | integer | `depth-limit="10000"` |
| svg-style | string | `max-width: 100%; height: auto;` |

### `defaults`

The `defaults` element is a container for providing element defaults.
Elements within the `defaults` block do not directly contribute to the final output, but provide default _attributes_ and _classes_ for matched elements.

Note that default value substitution happens early in the transformation process, and no attribute processing (e.g. variable lookup,
compound attribute expansion) is performed prior to populating a matching element.

Matches are controlled from two sources:

* The element name, or the element name '`_`' to match on any element type
* The `match` attribute, which is the sole attribute excluded from being a 'default'. This attribute is first split on comma-whitespace,
  and then used as selectors similar to (basic) CSS selectors, e.g. `rect` to match `<rect>` elements, `.my-class` to match elements with
  `my-class` as a class, and `circle.small` to match `<circle>` elements which also have the `small` class. No other selector types are
  supported at this point.

Any attributes on the matched element have priority over defaults.

If multiple matches occur, later matches override (in the case of attributes) or augment (for classes) earlier matches.

Two flag values can be provided in the `match` attribute:

* `init` causes any previous match information to be ignored, and this to be the starting point
* `final` prevents any further matching

Note both these flags only apply once a match has otherwise been made.

Note that defaults are scoped, typically through the use of the `<g>` element. More local scopes will take priority over outer scopes,
but do not replace them (though `<_ match="init"/>` at the start of a local `<defaults>` container would do this).

Note that the 'id' attribute cannot be defaulted.

Attributes which are effectively 'lists' are 'augmented', i.e. a local value or later matched element attribute is appended to earlier
ones, rather than replacing them. Augmented attributes include "class", "transform", "style", and "text-style". There is no attempt
at de-duplication except in the case of "class", which is special-cased throughout `svgdx`.

### `var`

This element allows one or more variables to be set. These values can be referenced later in [expressions](expressions#variables).

Variables are set using a `varname="value"` attribute pair, and multiple variables can be set in a single `<var>` element.

Note the `value` is considered to be an expressions, so variables can be set based on the value of other (or the same) variable.

Be careful when updating variable values; an element such as `<var thing="$thing + 1"/>` may appear to do the right thing in a document,
but internally if this is in a loop it will expand to a string of "... + 1 + 1 + 1 + 1 + 1 ...", which may work, but probably isn't
the intended effect, and will slow down document processing. (The likely correct approach here is to use `<var thing="{{$thing + 1}}"/>`.)
In order to help detect this string expansion, the config value `var-limit` (default 1024) limits the maximum length of string values
being assigned to variables.

### `specs`

This is a container element; the contents of it are not transferred to the rendered output, but may be referenced by other elements,
in particular the `reuse` element.

The element is analogous to SVG's `<defs>` element, in that "The ‘[specs]’ element is a container element for referenced elements ...
Elements that are descendants of a ‘[specs]’ are not rendered directly" [ref](https://www.w3.org/TR/SVG11/struct.html#DefsElement).
The difference is that `<defs>` remain in the document (and therefore DOM) at render time; `<specs>` do not.

Elements within a `<specs>` section should generally have an `id` attribute so they can be referenced, otherwise they will have no effect on the rendered document.

Note that `<specs>` elements may not be nested.

### `reuse`

The `<reuse>` element is analogous to SVG's `<use>` element, in that it takes an `href` attribute referring to another element.
The difference is that where a `<use>` element will remain as-is in the rendered output, the `<reuse>` element is _replaced_ by the referenced element.

Typically this is used to refer to elements defined in the `<specs>` section of the document, using a `href` attribute analogous to the `<use>` element.
(Note there should _not_ be an `xlink:` namespace prefix on the `href` attribute of `<reuse>` elements).

The `style` attribute of the `<reuse>` element is applied to the rendered output, as are any classes defined for the element.
Any `id` attribute of the `<reuse>` element is also applied to the rendered output element, and the target `id` becomes a new `class` entry.

For example:

```xml
<specs>
  <rect id="square" x="$x" y="$y" width="$size" height="$size"/>
</specs>
<reuse id="base" href="#square" x="0" y="0" size="10" class="thing"/>
```

will result in the following rendered output:

```xml
<rect id="base" x="0" y="0" width="10" height="10" class="thing square"/>
```

Any additional attributes on the `<reuse>` element are available in the target element's context as [local attribute variables](expressions#variable-references).

### `point`

The point element is used to define a position, via the `xy` - or separate `x` and `y` - attributes. It does not appear in the rendered output, and
is simply used to define a point which may later be referred to by other refspec attributes.

In general `<point>` elements will only be useful if they are given an `id` value.

Note that `<point>` elements differ from alternatives - such as a zero-width `<rect>` or zero-radius `<circle>` - by being ignored when composite
bounding boxes are being established, including the top-level SVG `viewBox`.

### `box`

The point element is used to define a rectangular region, via the `xy` - or separate `x` and `y` - attributes.
It does not appear in the rendered output, and is simply used to define a region which may later be referred to by other refspec attributes.

In general `<box>` elements will only be useful if they are given an `id` value.

Unlike the analogous `<point>` elements, `<box>` elements *do* contribute to any surrounding bounding box, and one use case is to define a surrounding
borderless region which other elements then sit within.

### `if`

The `<if>` element allows conditional inclusion of blocks of elements. A single attribute - `test` - provides the condition.
If the condition expression evaluates to non-zero, the contained block is processed as usual; if the condition evalates to zero then it is omitted.

Note the `test` expression is always evaluated in a numeric context - there is no need to surround the conditional expression with `{{..}}`.

Example:

```xml
<if test="eq($n, 7)">
  <text>Seven</text>
</if>
```

### `loop`

The `<loop>` element allows blocks of elements to be repeated. The repetition happens at the 'input' stage to processing,
so side-effects such as variable updates take effect in each repetition.

There are three forms of the loop element depending on given attribute:

* **`count`** - a fixed number of repeat counts.  The `count` variant of `<loop>` has three optional attributes: `var`, `start`, and `step`.
* **`while`** - this is given an expression as a condition, and iterations repeat **while** the condition is "true", which is defined as non-zero (as with the C language).
* **`until`** - similar to `while`, but the expression is evaluated at the *end* of each loop rather the start as with `while`, so will always be present at least once in the output.
