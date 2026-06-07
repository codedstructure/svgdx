# Delta 7 - Loops and Conditions

> `svgdx` allows elements to be included multiple times, or conditionally included.

## Loop elements

`svgdx` provides three forms of the `<loop>` element: **count**, **while**, and **until**. Each of these acts as a container, and all the elements nested within it are repeated based on the attributes defined on the loop element.

Every `<loop>` element must have exactly one of the attributes `count`, `while`, or `until`.

### Count-based loops

Count loops use the `count` attribute to define a fixed number of repeat counts.
Note the number of repeats is evaluated before any repeats are created, so while an expression (possibly including variables) can be provided to this attribute, it will only be evaluated once rather than each iteration.

For each iteration, all elements within the `<loop>` block are processed and appended to the output document.

Example:

```xml-svgdx
<svg>
  <var i="0"/>
  <loop count="4">
    <circle cxy="{{$i * 10}} 0" r="5"/>
    <var i="{{$i + 1}}"/>
  </loop>
</svg>
```

The `count` variant of `<loop>` has three optional attributes:

* **`var`** - variable name set on each loop iteration
* **`start`** - the initial value of the loop variable (default 0)
* **`step`** - the delta added to the variable on each iteration (default 1)

These provide shortcuts replacing a combination of `var` and `while`-based `<loop>`s.

Note that `start` and `step` are only meaningful if `var` is defined, and the number of iterations is always exactly the `count` value.
While `count` must always be a positive integer, `start` and `step` (and therefore the loop variable value) may be (possibly negative) floating point values.

> NOTE: If _expressions_ are given as `var`, `start` or `step` values, these are evaluated _once_ before the first loop iteration.

The above example may be re-written using these attributes as follows:

```xml-svgdx
<svg>
  <loop count="4" var="i" step="10">
    <circle cxy="$i 0" r="5"/>
  </loop>
</svg>
```

A fuller example, showing nested loops:

```xml-svgdx
<svg>
  <rect xy="0" wh="120 60"/>
  <loop count="2" var="i" step="30">
    <loop count="4" var="j" step = "30">
      <loop count="3" var="k" start="5" step="-1.5">
        <rect wh="20" xy="{{$j + $k}} {{$i + $k}}" class="d-softshadow"/>
      </loop>
    </loop>
  </loop>
</svg>
```

### While loops

**`while`** - this is given an expression as a condition, and iterations repeat **while** the condition is "true", which is defined as non-zero (as with the C language).

Example:

```xml-svgdx
<svg>
  <var x="0" y="0"/>
  <loop while="{{le($x, 90)}}">
    <var oldx="$x" oldy="$y" x="{{$x + 1}}" y="{{10 * sin($x * 10)}}"/>
    <line xy1="$oldx $oldy" xy2="$x $y"/>
  </loop>
</svg>
```

### Until loops

**`until`** - similar to `while`, but the expression is evaluated at the *end* of each loop rather the start as with `while`, so will always be present at least once in the output.

Example:

```xml-svgdx
<svg>
  <var size="30"/>
  <loop until="{{lt($size, 1)}}">
    <rect cxy="0" wh="$size" class="d-thinner"/>
    <var size="{{$size * 0.9}}"/>
  </loop>
</svg>
```

Note that for `while` and `until`, the expression is evaluated each iteration, whereas it is only evaluated once for the `count` form.

> NOTE: It is easy to generate very large documents using loops, and potentially take a long time to evaluate.
>
> To mitigate this, a separate **`loop-limit`** config value (default: 1000) is defined to detect excessive loop counts. If the number of loops exceeds this at any point, document processing is abandoned with an error.
>
> Note that `loop-limit` does not 'clamp' the number of loops, but is a limit which if exceeded rejects the input entirely.
> It is intended to detect and avoid infinite loops, which are easy to generate accidentally with malformed `while` and `until` conditions.
>
> As with other config elements, `loop-limit` can be set using the `<config>` element.

## Conditions

All conditions in `svgdx` are equivalent to a single check: is the value of a conditional expression *zero* (false) or *non-zero* (true). This is analogous to the C programming language, and related languages. Various functions such as `eq` (equals), `lt` (less than) or `ge` (greater than or equals) return a value of either 0 for false, or 1 (by convention) for true. Logical operation functions (`and`, `or`, `not` etc) operate similarly: each input is considered a condition and checked to see if they are non-zero, the logical operation applied, and either a one ('true') or zero ('false') returned.

## The `if` element

The `loop` elements above - specifically `while` and `until` loops - show a use of conditions, but a simple **`<if>`** container element is also provided by `svgdx`.  This has a single mandatory attribute '`test`', which is evaluated as a conditional expression. If true, everything contained in the `<if>` element is processed as normal; if the `test` condition is false, everything inside the `if` element is ignored.

```xml-svgdx
<svg>
  <config border="10"/>
  <var n="7"/>
  <text class="d-text">
    <tspan>7 is</tspan>
    <if test="eq($n % 2, 0)"><tspan>even!</tspan></if>
    <if test="eq($n % 2, 1)"><tspan>odd!</tspan></if>
  </text>
</svg>
```
