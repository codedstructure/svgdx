# **svgdx** - _create SVG diagrams easily_

**svgdx** is a command-line tool to convert a superset of SVG into an SVG image.

> **This project is in early development, there are many known issues and frequent feature updates.**
>
> In particular, the input format is **not stable** at this point. Check the [CHANGELOG](CHANGELOG.md) for info.

## Installation

For now installation requires a working Rust toolchain, e.g. installed from [rustup.rs](https://rustup.rs).

Install `svgdx` as follows:

    cargo install svgdx

## Usage

    svgdx [INPUT] [-o OUTPUT] [-w]

By default, `svgdx` reads from stdin and writes to standard output, so if run without any
arguments it simply waits for input.

The `-w` argument (which requires a non-stdin input file) 'watches' the input,
regenerating the output whenever it changes. This is particularly useful alongside
an SVG viewer / preview which also refreshes the view when the underlying file changes.

## Example

### Input

Prepare an input file ([examples/simple.xml](examples/simple.xml)):

```xml
<svg>
  <rect id="in" wh="20 10" text="input" />
  <rect id="proc" xy="^:h 10" wh="^" text="process" />
  <rect id="out" xy="^:h 10" wh="^" text="output" />

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

![](examples/simple.svg)

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
    text.d-tbox, text.d-tbox * { text-anchor: middle; dominant-baseline: central; }
    line.d-arrow, polyline.d-arrow, path.d-arrow { marker-end: url(#d-arrow); }
  </style>
  <rect id="in" width="20" height="10"/>
  <text x="10" y="5" class="d-tbox">input</text>
  <rect id="proc" x="30" y="0" width="20" height="10"/>
  <text x="40" y="5" class="d-tbox">process</text>
  <rect id="out" x="60" y="0" width="20" height="10"/>
  <text x="70" y="5" class="d-tbox">output</text>

  <line x1="20" y1="5" x2="30" y2="5" class="d-arrow"/>
  <line x1="50" y1="5" x2="60" y2="5" class="d-arrow"/>
</svg>
```

This example shows just a few of the features `svgdx` provides:

* shortcut attributes, e.g. the use of `wh` rather than having to specify `width` and `height` separately.
* relative positioning, either by reference to an id (e.g. `#in`), or to the previous element using the caret (`^`) symbol.
* new attributes providing additional functionality - `text` within shapes, or `start` and `end` points on `<line>` elements to define connectors.
* automatic calculation of root `<svg>` element `width`, `height` and `viewBox` derived from the bounding box of all elements.
* automatic styles added to provide sensible defaults for 'box and line' diagrams.

Many more features are provided by **svgdx**, with the goal of making a diagram something you can write, rather than draw.

See [more examples](examples/README.md)

## Background

SVGdx is intended to support 'typing a diagram' workflows, at a lower (and more flexible) level than structured tools such as [Mermaid](https://mermaid.js.org) or [Graphviz](https://graphviz.org).

An important principle is that raw SVG can be included directly at any point in the input. This is analogous to Markdown, which provides the 'escape hatch' of using inline HTML tags. Markdown has been incredibly successful as a text based format allowing simple text files to carry both semantic and simple style information. Being able to do both of those in a single workflow - just by _typing_ - allows a flow which would otherwise not be achievable.

Can the same be done for _drawing_ - specifically for **diagrams**? That's what **svgdx** aims to deliver.

Text files provide a number of advantages over other formats:
* Clear compatibility with version control - meaningful diffs
* Self-describing - text files are easy to (at least approximately) reverse engineer even in the absence of a clear spec or other tools
* Application independence - additional tools can be written to deal with the format
* Easy editing - at least for simple changes

### Text-based diagramming tools

There are several existing tools which convey diagrammatic information in textual form:

* [Mermaid](https://mermaid.js.org)
* [Graphviz](https://graphviz.org)
* [PlantUML](https://plantuml.com)

In most of these, the tools are specialised for various particular forms of data; they apply a degree of intelligence to the semantic content of the text and render diagrams representing this. For the special cases they work great, however adding additional layers of graphical structure beyond that imposed by the tool ends up fighting against it.

> _[Ditaa](https://ditaa.sourceforge.net) is different in that a diagram-structure is already provided as input, and it effectively just renders it. The work of creating the diagram has already been done._

When abstraction fails, moving to a lower layer is often the answer. Rather than starting with the input information, can we work backwards from the end result we want?

**[SVG](https://en.wikipedia.org/wiki/SVG)** is an XML-based language for defining vector images. The tools discussed above (with the exception of `ditaa`) can all output SVG images, and SVG is a (perhaps _the_)  lowest-common-denominator of graphics formats for diagramming tools.


### Why 'svgdx'?

Project naming is hard.

I could write about how this tool is about improving the **d**eveloper e**x**perience of creating diagrams; perhaps the **d**iagramming e**x**perience. But primarily the `dx` in `svgdx` is intended to refer to a __delta__ of SVG.

This is explicitly an _SVG_ tool, not some generic diagamming tool. It is most useful when combined with some experience of SVG.
While the SVG standards progress slowly, this tool allows keeping everything good from SVG and adding just _a little bit more_.
