# Expressions

Attribute values and text content

## Variables

### Variable naming
Variable names are alpha-numeric plus underscore. They may not start with a digit.

Variable names are case-sensitive; `abc` and `Abc` are two different variables.

### Variable references

Variable references are introduced with the `$` symbol.

`$abc` in an attribute value will be replaced with the content of the variable `abc`, assuming it exists.
An alternative format using braces may be used to avoid ambiguous variable references,
e.g. if `var` is defined as `1`, `${var}0` will expand to `10` (in non-[arithmetic](#arithmetic) contexts),
whereas `$var0` would be a reference to the (perhaps non-existent) `var0` variable.

While there is only a single global namespaces for variables, lookups are first done on the current element's attributes.
For example if an element defines has an `id="thing"` attribute, this may be referenced in the element's `text` attribute, as `text="Current element 'id' is: $id"`.

These "attribute locals" shadow global variables, but do not modify them.

Note that in the context of the [`reuse`](elements#reuse) element, the attributes of the `<reuse>` element itself provide the local attribute values, rather than the target element.

### Variable definition

Variables are defined in a custom `<var>` element, where each attribute
names and sets a variable. For example `<var key="1"/>` sets the variable `key` to be the value `1`.

Variables are untyped; there is a single global namespace, and modifying
variable values is performed by overriding the value.

Example - increment `var1`
```xml
<var var1="{{${var1} + 1}}" />
```

## Arithmetic

Arithmetic expressions are specified in double-brace pairs, for example `{{ 1 + $var }}`.

Expressions may include the following. Note these are listed in order of precedence.
* Numbers, including floating point and negative numbers. Internally numbers are stored with at least IEEE 754 single-precision floats, but exact precision and range are not part of this spec.
* Variable references of the form `$var` or `${var}`
* Element references, of the form `#id:v` where `id` indicates the target element and `v` is the value of that element to retrieve.
* `(`, `)` - parenthesis, for increasing precedence.
* `*`, `/`, `%` - multiply, divide, remainder. Precedence is left-to-right among these.
* `+`, `-` - addition and subtraction. Precedence is left-to-right among these.


## Element references

The following scalar values may be referred to from an element reference:

* `t`, `y`, `y1` - top, the y coordinate of the top of the given element
* `r`, `x2` - right, the x coordinate of the right-hand-side of the given element
* `b`, `y2` - bottom, the y coordinate of the bottom of the given element
* `l`, `x`, `x1` - left, the x coordinate of the left-hand-side of the given element
* `w`, `width` - the width of the given element
* `h`, `height` - the height of the given element
* `cx` - the x coordinate of the centre of the given element
* `cy` - the y coordinate of the centre of the given element
* `rx` - the x-radius of the given element
* `ry` - the y-radius of the given element

These are accessed by providing an element reference (e.g. `#abc`) followed by a
dot (`.`), followed by the appropriate entry from the list above.

Note these are different to the relative locations which may be derived from an element.

> NOTE: this currently has two overlapping use-cases:
>
> * get scalar geometric values from an element
> * get the (numeric) value from an attribute of an element
>
> In many cases these are equivalent, but in some cases they can have different
> meanings. For example: `rx` on a `<rect>` vs an `<ellipse>`, or `x2` to mean
> the right-hand side of an element - when > a `<line>` may have an `x2` attribute
> less than its `x1` value.
>
> The cleanest way to resolve this is likely splitting up the ScalarSpec type.
