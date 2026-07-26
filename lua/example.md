# Example

This file shows the two main ways to use the filter.

Inline output is convenient for HTML-style documents:

```svgdx
<svg>
	<rect wh="20" class="d-fill-red"/>
</svg>
```

For PDF, docx, and other outputs that need an image file, add `output=`:

```{.svgdx output="images/green-circle.svg"}
<svg>
	<circle r="10" class="d-fill-green"/>
</svg>
```

## Run it

Create the output directory first, then run pandoc with the filter:

```sh
mkdir -p out/images
pandoc --lua-filter <(svgdx --pandoc-lua-filter) example.md -o out/output.html
```

That command:

- embeds the red rectangle directly in the HTML output
- writes `out/images/green-circle.svg`
- links to `images/green-circle.svg` in linked outputs such as HTML and Markdown
- uses an absolute image path internally for embedded outputs such as PDF and docx
