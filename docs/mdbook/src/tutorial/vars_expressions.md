# Delta 6 - Variables and Expressions

## Variables

Variables are **defined** in svgdx in several ways, but primarily through the `<vars>` element.

Variable **names** begin with an alphabetic or underscore character, optionally followed by
alphanumeric or underscore characters. The variable value `_` should be avoided, as it has
meaning as a 'comment attribute', and cannot be defined via the `<vars>` element.
Note that variable names are case sensitive.

Variables are **referenced** using the '$' symbol, followed by the variable name.
If a variable reference is followed by alphanumeric characters which could

### Examples

Defining and referencing variables:

```xml-svgdx-inline
<svg>
  <var abc="123"/>
  <rect wh="20 10" text="$abc"/>
  <!-- Note use of ${...} to delimit the var name -->
  <rect xy="^|v 2" wh="20 10" text="${abc}4"/>
</svg>
```

Multiple variables can be defined in a single `<vars>` element; each variable is assigned the attribute's value.

```xml-svgdx-inline
<svg>
  <var size="20 10" col="red"/>
  <rect wh="$size" text="Hello!" class="d-text-$col"/>
</svg>
```

Variable values can be updated after being set, and can the new value can include variable references,
including the value itself.

```xml-svgdx-inline
<svg>
  <var size="20 10" msg="Hello "/>
  <rect xy="0" wh="$size" text="$msg"/>
  <var msg="${msg}World"/>
  <rect xy="0" wh="$size" text="$msg"/>
</svg>
```

Variables defined in a `<var>` element are updated simultaneously in parallel, allowing variables to be swapped in a single var element:

```xml-svgdx-inline
<svg>
  <var a="1" b="2"/>
  <rect xy="0" wh="20 10" text="a = $a; b = $b"/>
  <var a="$b" b="$a"/>
  <rect xy="^|v 2" wh="20 10" text="a = $a; b = $b"/>
</svg>
```

## Expressions

All the examples above treat the variables as simple string substitution. When included in an **expression block**,
delimited with double-braces (`{{...}}`), expressions including variables can be evaluated.

```xml-svgdx-inline
<svg>
  <var a="1" b="2"/>
  <!-- text substitution by default -->
  <rect xy="0" wh="20 10" text="$a + $b"/>
  <!-- evaluated as expression inside '{{...}}' -->
  <rect xy="^|v 2" wh="20 10" text="{{$a + $b}}"/>
</svg>
```

The standard operators `+`, `-`, `*` and `/` are supported, as well as `%` for modulo.
Standard arithmetic operator precedence takes effect, including support for parenthesized expressions.

```xml-svgdx-inline
<svg>
  <var a="3" b="5"/>
  <rect id="a" xy="0" wh="15 10" text="a = $a"/>
  <rect xy="^|v 2" wh="15 10" text="a+b = {{$a + $b}}"/>
  <rect xy="^|v 2" wh="15 10" text="a-b = {{$a - $b}}"/>
  <rect xy="#a|h 2" wh="15 10" text="b = $b"/>
  <rect xy="^|v 2" wh="15 10" text="a*b = {{$a * $b}}"/>
  <rect xy="^|v 2" wh="15 10" text="a/b = {{$a / $b}}"/>
</svg>
```

## Built-in functions

A range of functions are provided by svgdx, which are called using `fn(arg1, arg2, ...)` syntax.
The result of the function is substituted into the expression as-is. Note variable expansion
happens first when evaluating expressions, so the function to be used can be provided by a variable,
for example:

```xml-svgdx-inline
<svg>
  <var x="45" fn="sin"/>
  <circle r="10" text="{{$fn($x)}}"/>
</svg>
```

> There is currently no support for user-defined functions in svgdx;
> this may be added in future.

Functions include the following:

* **Trigonometric functions** `sin`, `cos`, `tan`, `asin`, `acos`, `atan`; note these are all based on _degrees_,
  not the radians which most programming languages use.
* **Polar / Rectangular conversion** `r2p`, `p2r` - convert between a pair of values in rectangular and polar coordinates.
  Note that '0 degrees' points horizontally to the right, and as the angle increases it turns clockwise,
  such that `p2r(1, 90)` is `0, -1`.
* **Random number generation** `random()` produces a floating point value in the range 0..1.
  `randint(a, b)` produces an integer in the _inclusive_ range \[a..b\].
* **Exponential and logarithmic** `pow(x, y)` (x<sup>y</sup>), `exp(x)` (e<sup>x</sup>), `log(x)` (natural log of x)
* TODO: many more!

## Types in expressions

Values in expressions can be numbers, strings, or lists of these types.

Numbers are internally stored as 32 bit IEEE754 floating point values, with integers up to several million stored exactly.
Note that while final attribute numeric values are aggressively rounded for human comprehension,
within an expression (and in variable values) full precision is maintained.

There is no specific boolean type, with zero being considered 'false' and any other value 'true'.

There is no explicit syntax for lists; rather comma-separated values are considered a list.
Therefore a function's arguments (if more than one) are a list, and a function may return a list,
which may be substituted into another function, or used as the value of an attribute which takes
a comma separated list.

```xml-svgdx-inline
<svg>
  <var x="45" fn="sin"/>
  <circle r="10" text="{{p2r(10, -30)}}"/>
  <circle r="2" cxy="{{p2r(10, -30)}}"/>
  <box wh="12 4" xy="^|h" text="{{r2p(p2r(10, -30))}}"/>
</svg>
```
