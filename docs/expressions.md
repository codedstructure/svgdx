# Expressions

Attribute values and text content

## Variables

### Variable naming
Variable names are alpha-numeric plus underscore. They may not start with a digit.

Variable names are case-sensitive; `abc` and `Abc` are two different variables.

### Variable references

Variable values are introduced with the `$` symbol.
`$abc` in an attribute value will be replaced with the content of the variable `abc`, assuming it exists.
An alternative format using braces may be used to avoid ambiguous variable references,
e.g. if `var` is defined as `1`, `${var}0` will expand to `10` (in non-[arithmetic](#arithmetic) contexts),
where `$var0` would be a reference to the non-existent `var0` variable.

### Variable definition

Variables are defined in a custom `<define>` element, where each attribute
names and sets a variable. For example `<define var="1"/>` sets the variable `var` to be the value `1`.

Variables are untyped; there is a single global namespace, and modifying
variable values is performed by overriding the value.

Example - increment `var1`
```xml
<define var1="{{${var1} + 1}}" />
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

* `t` - top, the y coordinate of the top of the given element
* `r` - right, the x coordinate of the right-hand-side of the given element
* `b` - bottom, the y coordinate of the bottom of the given element
* `l` - left, the x coordinate of the left-hand-side of the given element
* `w` - the width of the given element
* `h` - the height of the given element

Note these are different to the relative locations which may be derived from an element.
