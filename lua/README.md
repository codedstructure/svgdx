# svgdx Pandoc Lua filter

This directory contains a pandoc Lua filter to convert Markdown containing
fenced `svgdx` blocks to SVG images linked or embedded in the output document.

## Requirements

- [Pandoc](https://pandoc.org) 3+
- [svgdx](https://svgdx.net) on your `PATH`

The `SVGDX_BIN` environment variable may point to a specific `svgdx` executable,
avoiding the need to have svgdx on your PATH.

## Invoking the filter from pandoc

The Lua filter is embedded within the `svgdx` command line binary, so it may be
used with Bash process substitution when running pandoc:

```sh
pandoc --standalone --lua-filter <(svgdx --pandoc-lua-filter) input.md -o output.html
```

If you are working from a this project's source code, point pandoc at the Lua
file instead:

```sh
pandoc --standalone --lua-filter lua/svgdx-pandoc-filter.lua input.md -o output.html
```

By default, `svgdx`-fenced code blocks are converted to inline SVG within
generated output. This is useful for HTML and Markdown output.

## Use `output=` when pandoc needs a real image file

For PDF, docx, and any other embedded-document workflow, give the fence an
`output` attribute so the filter writes an SVG file and inserts an image
reference:

````markdown
```{.svgdx output="images/hello.svg"}
<svg>
  <rect wh="20 10" text="Hello svgdx"/>
</svg>
```
````

> NOTE: \```{.svgdx output="thing.svg"} is equivalent to \```svgdx {output="thing.svg"}

This may also be useful in 'linked' formats such as Markdown or HTML when standalone image files are more suitable.

The filter resolves a relative `output=` path from the output document's directory. That keeps the generated SVG next to the document you are building rather than next to the source Markdown.

- When pandoc writes a linked format such as HTML or Markdown to a file, the
  image link stays relative, so moving the document and its `images/` directory
  together still works.
- When pandoc builds an embedded format such as PDF or docx, or when pandoc
  writes to stdout, the image link becomes absolute so pandoc and its helper
  tools can still find the SVG.
- If `output=` is already absolute, that exact path is used.
- Any parent directories of specified output files must already exist, but note
  that existing files will be overwritten.

See [example.md](./example.md) for a short input file that demonstrates both
inline output and use of the `output=` attribute.
