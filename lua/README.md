# svgdx Pandoc Lua filter

This repository contains a [Pandoc](https://pandoc.org) 3+ Lua filter for Markdown documents with `svgdx` fenced code blocks. Released `svgdx` builds expose the checked-in filter via `svgdx --pandoc-lua-filter`; from a repository checkout you can also point pandoc directly at `lua/svgdx-pandoc-filter.lua`.

## Requirements

- **[Pandoc](https://pandoc.org)** 3.0 or later
- **[svgdx](https://svgdx.net)** on your `PATH`

## Basic usage

```sh
pandoc --standalone --lua-filter <(svgdx --pandoc-lua-filter) input.md -o output.html
```

If you are working from a checkout of this repository, the equivalent direct file path is:

```sh
pandoc --standalone --lua-filter lua/svgdx-pandoc-filter.lua input.md -o output.html
```

Inline SVG is the default. Use a normal `svgdx` fence:

````markdown
```svgdx
<svg>
  <rect wh="20 10" text="Hello svgdx"/>
</svg>
```
````

The filter runs the block through `svgdx` and embeds the resulting SVG directly in the output document. In inline mode it strips blank lines from the generated SVG so that embedded HTML stays valid inside Markdown.

See the [example document](./example.md) for a complete example.

> By default the filter runs `svgdx` on the PATH, but if the `SVGDX_BIN` environment variable is set it will be used instead.
>
> ```sh
> SVGDX_BIN=/opt/custom/bin/svgdx pandoc --standalone --lua-filter <(svgdx --pandoc-lua-filter) input.md -o output.html
> ```

## Writing SVG files

To write a rendered diagram to a specific SVG file instead of inlining it, use Pandoc's standard fenced-code attributes:

````markdown
```{.svgdx output="images/hello.svg"}
<svg>
  <rect wh="20 10" text="Hello svgdx"/>
</svg>
```
````

When `output` is present, the filter overwrites that file with the rendered SVG and returns an image link pointing at the same path.

- The parent directory must already exist.
- Relative paths are resolved from pandoc's current working directory.
- The filter always writes SVG bytes; it does not inspect the extension.
- Explicit output preserves the rendered SVG line structure; blank-line stripping is only applied to inline output.

This version deliberately does not generate PNG files or manage temporary files. If you need PNG or PDF derivatives, run a separate post-processing step over the generated SVG files.

## Error handling

- If `svgdx` is not available when the filter loads, pandoc stops immediately with an error.
- If processing a block fails, the filter writes a warning to stderr and inserts a red-bordered HTML error block at that point in the document.
- If writing an explicit `output` file fails, the filter reports it the same way.
