# **svgdx** - _create SVG diagrams easily_

> **svgdx** is a
> diagrams-as-code **format** extending SVG,
> as well as a **library** and **tools** to convert this SVG superset into an SVG image.

The goal of svgdx is to allow (1) _arbitrary_ (2) _diagrams_ to be (3) _directly written_ as code:

1. **arbitrary**, rather than restricted to a set of specific use-cases
2. **diagrams**, rather than free-form images, because *some* constraint is useful
3. **directly written**, rather than requiring an image editor

<p align="center">
  <img width="85%" src="examples/svgdx.svg?raw=true"/>
</p>

## Why another diagram-as-code language?
**(or, why _svgdx_ might not be for you)**

There are many diagram-as-code ('DaC') languages and tools, including [Graphviz][], [PlantUML][], [Mermaid][], and [D2][].

[Graphviz]: https://graphviz.org
[PlantUML]: https://plantuml.com
[Mermaid]: https://mermaid.js.org
[D2]: https://d2lang.com

What these have in common is they tend to be **high level**, automating the leap from the represented domain to a visual representation.
The trade-off of being high-level is sacrificing some level of control over diagram layout.
In many cases this is worthwhile, or even desirable - the language and tooling might be intended to _reveal_ layout (e.g. traditional Graphviz applications), rather than encode it.

There are cases where a specific layout (or other aspects) is already intended.
If _"the medium is the message"_, then in diagrams _structure_ and _layout_ can provide clarity which an automated translation cannot always provide.

Consider a diagram scribbled on a napkin. How should that be encoded for computer use?

Option 1: **Just take a photo**

Option 2: Use something like [Inkscape][], [Dia][], [draw.io][] or [Visio][] to **"draw" the diagram on a computer**

[Inkscape]: https://inkscape.org
[Dia]: https://wiki.gnome.org/Apps/Dia
[draw.io]: https://drawio.com
[Visio]: https://www.microsoft.com/microsoft-365/visio/flowchart-software

Option 3: Quickly **code it using svgdx!**

## Selected feature highlights

* Relative positioning of SVG shapes
* Template-like fragment re-use
* Variables, expressions, conditionals and loops
* Web-based editor, pandoc and mdBook plugins available

Most importantly, `svgdx` is a _superset_ of SVG, so any valid SVG is also valid as input to an svgdx processor.
This distinguishes `svgdx` from many other diagrams-as-code languages, which are typically use-case driven.
These other tools are typically easier to use when 'going with the grain', but sacrifice control and flexibility.
In contrast, anything possible in SVG is (by definition) also possible in `svgdx`.

One big caveat first:

> **This project is in active pre-v1.0 development, with known issues and frequent changes.**
>
> In particular, **the input format is not stable** at this point, lacking even an informal specification.
>
> Stability and backwards compatibility will become much higher priority once the project reaches v1.0.0.
> Check [CHANGELOG.md](CHANGELOG.md) for more info.

and a smaller caveat:

> `svgdx` benefits from a basic knowledge of [SVG][] and [XML][].
> This will help avoid surprises when everything seems to work fine until you want an ampersand in your text...

[SVG]: https://developer.mozilla.org/en-US/docs/Web/SVG
[XML]: https://developer.mozilla.org/en-US/docs/Web/XML/XML_introduction

## Try _svgdx_ online

Try **svgdx** in the online editor at **[svgdx.net][]**, where svgdx can be explored without installing anything.
This repository includes a local server implementing the same editor, allowing fully offline use.

_note that the svgdx.net editor is whenever a new version of svgdx is tagged,
so be aware that breaking changes may affect your diagrams._

Integrations with **[mdBook][]** and **[pandoc][]** are supported via the plugins [mdbook-svgdx][] and [svgdx-pandoc][] respectively;
these both provide support for Markdown documents containing `svgdx`-fenced code-blocks, which are processed and rendered as inline SVG images.

[svgdx.net]: https://svgdx.net
[mdBook]: https://rust-lang.github.io/mdBook/
[pandoc]: https://pandoc.org
[mdbook-svgdx]: https://github.com/codedstructure/mdbook-svgdx
[svgdx-pandoc]: https://github.com/codedstructure/svgdx-pandoc

## Installation

For now installation requires a working Rust toolchain, e.g. installed from [rustup.rs](https://rustup.rs).

Install `svgdx` as follows:

    cargo install svgdx

## Usage

After installation, two binaries are available:

### svgdx

    svgdx [INPUT] [-o OUTPUT] [-w]

By default, `svgdx` reads from stdin and writes to standard output, so if run without any
arguments it simply waits for input.

The `-w` argument (which requires a non-stdin input file) 'watches' the input,
regenerating the output whenever it changes. This is particularly useful alongside
an SVG viewer / preview which also refreshes the view when the underlying file changes.

### svgdx-server & editor

    svgdx-server --open

Running `svgdx-server` starts an HTTP server on localhost port 3003.

This provides an `/api/transform` endpoint; when a valid svgdx document is POSTed to this (as `application/xml`),
the generated `svg+xml` response will be returned.

More immediately useful, the `--open` argument causes a browser to open serving the same editor as running on [https://svgdx.net](https://svgdx.net).
There are minor differences, in that the hosted version uses WASM rather than a backend server to perform conversion in the browser,
but the entire web app (including vendored third-party libraries) is included within the `svgdx-server` binary.

## Example

### Input

Prepare an input file ([examples/simple.xml](examples/simple.xml)):

```xml
<svg>
  <rect id="in" wh="20 10" text="input" />
  <rect id="proc" xy="^|h 10" wh="^" text="process" />
  <rect id="out" xy="^|h 10" wh="^" text="output" />

  <line start="#in" end="#proc" class="d-arrow"/>
  <line start="#proc" end="#out" class="d-arrow"/>
</svg>
```

### Processing

Process the input with `svgdx`:

```bash
$ svgdx examples/simple.svg -o examples/simple.svg
```

### Output
Output file ([examples/simple.svg](examples/simple.svg)):

<p align="center">
  <img width="85%" src="examples/simple.svg?raw=true"/>
</p>

which is a rendering of the following generated SVG:

```xml
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="96mm" height="22mm" viewBox="-8 -6 96 22">
  <defs>
    <marker id="d-arrow" refX="1" refY="0.5" orient="auto-start-reverse" markerWidth="5" markerHeight="5" viewBox="0 0 1 1">
      <path d="M 0 0 1 0.5 0 1" style="stroke-width: 0.2; stroke: context-stroke; fill: context-fill; stroke-dasharray: none;"/>
    </marker>
  </defs>
  <style>
    rect, circle, ellipse, line, polyline, polygon, path { stroke-width: 0.5; stroke: black; fill: none; }
    text { font-family: sans-serif; font-size: 3px; }
    text.d-text, text.d-text * { text-anchor: middle; dominant-baseline: central; }
    line.d-arrow, polyline.d-arrow, path.d-arrow { marker-end: url(#d-arrow); }
  </style>
  <rect id="in" width="20" height="10"/>
  <text x="10" y="5" class="d-text">input</text>
  <rect id="proc" x="30" y="0" width="20" height="10"/>
  <text x="40" y="5" class="d-text">process</text>
  <rect id="out" x="60" y="0" width="20" height="10"/>
  <text x="70" y="5" class="d-text">output</text>

  <line x1="20" y1="5" x2="30" y2="5" class="d-arrow"/>
  <line x1="50" y1="5" x2="60" y2="5" class="d-arrow"/>
</svg>
```

This example shows just a few of `svgdx`'s features:

* shortcut attributes, e.g. the use of `wh` rather than having to specify `width` and `height` separately.
* relative positioning, either by reference to an id (e.g. `#in`), or to the previous element using the caret (`^`) symbol.
* new attributes providing additional functionality - `text` within shapes, or `start` and `end` points on `<line>` elements to define connectors.
* automatic calculation of root `<svg>` element `width`, `height` and `viewBox` derived from the bounding box of all elements.
* automatic styles added to provide sensible defaults for 'box and line' diagrams.

Many more features are provided by **svgdx**, with the goal of making a diagram something you can write, rather than draw.

See [more examples](examples/README.md)

## Background

An important principle is that raw SVG can be included directly at any point in the input. This is analogous to Markdown, which provides the 'escape hatch' of using inline HTML tags (used in this very README!). Markdown has been incredibly successful as a text based format allowing simple text files to carry both semantic and simple style information. Being able to do both of those in a single workflow - just by _typing_ - allows a flow which would otherwise not be achievable.

Can the same be done for _drawing_ - specifically for **diagrams**? That's what **svgdx** aims to deliver.

Text files provide a number of advantages over other document formats:

* Clear compatibility with version control - meaningful diffs
* Self-describing - text files are easy to (at least approximately) reverse engineer even in the absence of a clear spec or other tools
* Application independence - additional tools can be written to deal with the format
* Easy editing - at least for simple changes

### Why 'svgdx'?

Project naming is hard.

I could write about how this tool is provides a set of **svg** **d**iagram e**x**tensions. But primarily and originally, the `dx` in `svgdx` is intended to refer to a _delta_ of SVG.

This is explicitly an _SVG_ tool, not some generic diagamming tool. It is most useful when combined with some experience of SVG.
While the SVG standards progress slowly, this tool allows keeping everything good from SVG and adding _just that little bit more_.
