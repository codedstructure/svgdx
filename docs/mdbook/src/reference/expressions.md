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

While there is only a single global namespaces for variables, lookups are first done on the attributes of
any parent element's attributes.
For example if a `<g>` element defines a `radius="3"` attribute, this may be referenced in attributes of
the element's children, e.g. `<rect rx="$radius" .../>`.
Note there is no way to refer to attributes of the current element, as that would allow circular references -
an element such as `<g width="4"><rect width="$width" ...></g>` would not work as expected, and starting
attribute lookup at the parent element avoids the need to be 'creative' with variable names.

These "attribute locals" shadow global variables, but do not modify them.

> Note that in the context of the [`reuse`](elements#reuse) element, the attributes of the `<reuse>` element itself provide the local attribute values, rather than the target element.

> Note that as an exception, the [`var`](elements#var) element does not provide access to its attributes as locals, as that would cause infinite recursion when redefining a variable in terms of itself.

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
* Element references, of the form `#id~v` where `id` indicates the target element and `v` is the value of that element to retrieve.
* [function](#built-in-functions) calls, of the form `function(args)`
* `(`, `)` - parenthesis, for increasing precedence.
* `*`, `/`, `%` - multiply, divide, remainder. Precedence is left-to-right among these.
* `+`, `-` - addition and subtraction. Precedence is left-to-right among these.
* `,` - expression separator.

## Multiple expressions

Note that multiple expressions may be provided within a single `{{...}}` pair, and must be comma-separated.
This allows attributes such as `wh="{{$t + 3, $t + 2}}"` rather than the (slightly) more verbose `wh="{{$t + 3}} {{$t + 2}}"`.

Note that *input* expressions must be **comma** separated, and the resulting list of expression results are separated with `", "`.
Most SVG attributes which take multiple values use the `comma-wsp` format, where commas are optional and whitespace may be used to separate values,
but allowing whitespace-only separation for multiple expressions makes errors more likely.

## Built-in functions

A selection of built-in functions are provided, as follows:

| function | description |
| --- | --- |
| `abs(x)` | absolute value of x |
| `ceil(x)` | ceiling of x |
| `floor(x)` | floor of x |
| `fract(x)` | fractional part of x |
| `sign(x)` | -1 for x < 0, 0 for x == 0, 1 for x > 0 |
| `sqrt(x)` | square root of x |
| `log(x)` | (natural) log of x |
| `exp(x)` | raise e to the power of x |
| `pow(x, y)` | raise x to the power of y |
| `sin(x)` | sine of x (x in degrees) |
| `cos(x)` | cosine of x (x in degrees) |
| `tan(x)` | tangent of x (x in degrees) |
| `asin(x)` | arcsine of x degrees |
| `acos(x)` | arccosine of x in degrees |
| `atan(x)` | arctangent of x in degrees |
| `random()` | generate uniform random number in range 0..1 |
| `randint(min, max)` | generate uniform random integer in range [min, max] inclusive |
| `min(a, b)` | minimum of two values |
| `max(a, b)` | maximum of two values |
| `clamp(x, min, max)` | return x, clamped between min and max |
| `mix(start, end, amount)` | linear interpolation between start and end |
| `eq(a, b)` | 1 if a == b, 0 otherwise |
| `ne(a, b)` | 1 if a != b, 0 otherwise |
| `lt(a, b)` | 1 if a < b, 0 otherwise |
| `le(a, b)` | 1 if a <= b, 0 otherwise |
| `gt(a, b)` | 1 if a > b, 0 otherwise |
| `ge(a, b)` | 1 if a >= b, 0 otherwise |
| `if(cond, a, b)` | a if cond is non-zero, else b |
| `not(a)` | 1 if a is zero, 0 otherwise |
| `and(a, b)` | 1 if both a and b are non-zero, 0 otherwise |
| `or(a, b)` | 1 if either a or b are non-zero, 0 otherwise |
| `xor(a, b)` | 1 if either a or b are non-zero but not both, 0 otherwise |

Note these functions (e.g. the order of arguments in `mix` and `clamp`) are influenced by GLSL.

> Unlike most programming languages, **degrees** are the unit used for trigonometric functions.

## Element references

The following scalar values may be referred to from an element reference:

* `x`, `x1` - the x coordinate of the left-hand-side of the given element
* `y`, `y1` - the y coordinate of the top of the given element
* `x2` - the x coordinate of the right-hand-side of the given element
* `y2` - the y coordinate of the bottom of the given element
* `w`, `width` - the width of the given element
* `h`, `height` - the height of the given element
* `cx` - the x coordinate of the centre of the given element
* `cy` - the y coordinate of the centre of the given element
* `r` - the radius of the given element (assuming a circle!)
* `rx` - the x-radius of the given element
* `ry` - the y-radius of the given element

These are accessed by providing an element reference (e.g. `#abc`) followed by a
tilde (`~`), followed by the appropriate entry from the list above.

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
