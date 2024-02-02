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
| add-auto-styles | bool | `auto-add-styles="false"` | Inverse of `--no-auto-style` CLI option |
| background | [colour name](https://www.w3.org/TR/SVG11/types.html#ColorKeywords) | `background="lightgrey"` |
| scale | float | `scale="2.5"` |
| border | integer | `border="20"` |

### `var`

This element allows one or more variables to be set. These values can be referenced later in [expressions](expressions#variables).

Variables are set using a `varname="value"` attribute pair, and multiple variables can be set in a single `<var>` element.

Note the `value` is considered to be an expressions, so variables can be set based on the value of other (or the same) variable.
