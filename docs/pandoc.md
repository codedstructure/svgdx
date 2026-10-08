# Using svgdx with pandoc

The pandoc Lua filter built in to `svgdx` allows diagrams to be embedded
directly within Markdown documents in the form of fenced code blocks.

> This file is intentionally a self-contained example.
> If you run the commands [below](#worked-example) on this file, it will
> generate its own rendered output and image assets.

Three approaches to conversion are supported:

### inline SVG

This works well for conversion to HTML documents with embedded SVG content. It
will *not* work for many other formats though, so this is probably best for
contexts where HTML output is the only document target. In that case, it has the
virtue of transforming a single input document to a single output document -
there are no links to break.

Generate inline SVG by simply providing the 'svgdx' class to the fenced code block.

### external SVG image file

By providing a filename parameter with the '.svg' extension, the filter will
write an SVG file to the given filename (relative to the output document) and
include an image link in the generated document.

### external PNG image file

A filename parameter with the '.png' extension will be piped through the
`rsvg-convert` tool (which must be installed) in order to write a PNG file. This
is the most compatible, but lacks the scalability and small file size benefits
of the SVG format. Note that the PNG file is rendered at 300 DPI to provide
reasonable fidelity.


## Requirements

- [svgdx](https://svgdx.net) CLI available (installed or built in this repository)
- [pandoc](https://pandoc.org) version 3.1 or later
- [rsvg-convert](https://gitlab.gnome.org/GNOME/librsvg/-/tree/main/rsvg_convert) (required when emitting `.png` files; install from `librsvg` (homebrew) or `librsvg2-bin` (apt) packages.)

Quick checks:

```bash
svgdx --version
pandoc --version
rsvg-convert --version
```

If you only use inline output or `.svg` file output, `rsvg-convert` is not required.

## Usage

Export the embedded filter to a local file:

```bash
mkdir -p pandoc-html
svgdx --output-pandoc-filter > svgdx-pandoc.lua
```

This can then be used with pandoc's `--lua-filter` flag.

```bash
pandoc --lua-filter svgdx-pandoc.lua ...
```

The lua filter will extract relevant fenced code blocks and pass them through
svgdx, which should either be in the `PATH`, or defined by the `SVGDX_BIN`
environment variable.

If using a shell such as bash, process substitution can be used to avoid the
need to write the filter to a separate file, though a separate file may be
useful for making modifications.

```bash
pandoc --lua-filter <(svgdx --output-pandoc-filter) ...
```

## Markdown fenced code block syntax

The filter processes fenced code blocks with class `svgdx`:

````markdown
```svgdx
<svg>...</svg>
```
````

The optional `filename` parameter controls file output type, with .svg or .png
extensions supported.

````markdown
```svgdx { filename="images/diagram.svg" }
<svg>...</svg>
```
````

## Worked example

This document is itself an example of using the svgdx pandoc filter. From the
repository root, run the following:

```bash
mkdir -p docs/tmp/pandoc-html
pandoc \
  --from markdown \
  --to html \
  --standalone \
  --lua-filter <(svgdx --output-pandoc-filter) \
  docs/pandoc.md \
  --output docs/tmp/pandoc-html/pandoc.html
```

After running the commands in this document, open the
/docs/tmp/pandoc-html/pandoc.html file in a browser.

Expected results - note these links will only work in the rendered version!

- [pandoc.html](pandoc.html) exists
- [images/generated-svg.svg](images/generated-svg.svg) exists
- [images/generated-png.png](images/generated-png.png) exists

### 1. Inline output (`svgdx`, no filename)

```svgdx
<svg>
  <rect wh="40 16" text="inline svg" class="d-text-italic d-text-larger d-red d-fill-grey"/>
</svg>
```

### 2. File output as SVG

This writes an SVG file to `pandoc-html/generated-svg.svg` relative to this
source file directory (`docs/`), and references it in the rendered document.

```svgdx {filename="images/generated-svg.svg"}
<svg>
  <defaults><_ class="d-text-larger d-text-ol d-fill-lightblue"/></defaults>
  <rect id="a" wh="40 20" text="A" />
  <rect id="b" xy="#a|h 6" wh="^" text="B" />
  <line start="#a" end="#b" class="d-arrow" />
</svg>
```

### 3. File output as PNG

This writes a PNG file to `pandoc-html/generated-png.png` and references it in
the rendered document.

```svgdx {filename="images/generated-png.png"}
<svg>
  <ellipse cxy="30 10" wh="80 35" />
  <ellipse cxy="10 10" wh="^ -10 -10" text="PNG"/>
  <ellipse cxy="50 10" wh="^ -5 -5" text="svgdx!"/>
</svg>
```

## Notes

- Inline mode returns raw HTML `<svg>`, so it is intended for HTML output targets.
- Inline mode is not suitable for LaTeX/PDF generation (`pandoc -t pdf`): raw
  HTML blocks are not rendered as PDF graphics.
- File-backed modes (`.svg` or `.png`) are more portable across non-HTML targets
  because they become regular image references.
- PNG output requires `rsvg-convert`; missing it will fail conversion when
  `.png` examples are present.
- Image file paths are resolved relative to the output file location, not your
  shell working directory.
