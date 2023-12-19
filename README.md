# svgdx

**svgdx** is a 'delta' on SVG; a preprocessor and transpiler which accepts a superset of SVG and outputs SVG.

It is intended to support 'typing a diagram' workflows, at a lower (and more flexible) level than structured tools such as [Mermaid](https://mermaid.js.org) or [Graphviz](https://graphviz.org).
An important principle is that raw SVG can be included directly at any point in the input. This is analogous to Markdown, which provides the 'escape hatch' of using inline HTML tags. Markdown has been incredibly successful as a text based format allowing simple text files to carry both semantic and simple style information. Being able to do both of those in a single workflow - just by _typing_ - allows a flow which would otherwise not be achievable.

Can the same be done for _drawing_ - specifically for **diagrams**? That's what **svgdx** aims to deliver.

Text files provide a number of advantages over other formats:
* Clear compatibility with version control - meaningful diffs
* Self-describing - text files are easy to (at least approximately) reverse engineer even in the absence of a clear spec or other tools
* Application independence - additional tools can be written to deal with the format
* Easy editing - at least for simple changes

There are several existing tools which convey diagrammatic information in textual form:

* [Mermaid](https://mermaid.js.org)
* [Graphviz](https://graphviz.org)
* [PlantUML](https://plantuml.com)

In most of these, the tools are specialised for various particular forms of data; they apply a degree of intelligence to the semantic content of the text and render diagrams representing this. For the special cases they work great, however adding additional layers of graphical structure beyond that imposed by the tool ends up fighting against it.

> _[Ditaa](https://ditaa.sourceforge.net) is different in that a diagram-structure is already provided as input, and it effectively just renders it. The work of creating the diagram has already been done._

When abstraction fails, moving to a lower layer is often the answer. Rather than starting with the input information, can we work backwards from the end result we want?

**[SVG](https://en.wikipedia.org/wiki/SVG)** is an XML-based language for defining vector images. The tools discussed above (with the exception of `ditaa`) can all output SVG images, and SVG is a (perhaps _the_)  lowest-common-denominator of graphics formats for diagramming tools.

SVG is a fairly simple language, and a small subset can be used to create a large number of diagrams:

* Rectangles
* Lines
* Text

Out of these, text and positioning are frustrating in SVG, but it's still straightforward to make simple diagrams.

```xml
<svg>
  <rect x="10" y="2" width="20" height="10" />
  <text x="11" y="8">input</text>

  <line x1="30" y1="7" x2="40" y2="7" />

  <rect x="40" y="2" width="20" height="10" />
  <text x="41" y="8">process</text>

  <line x1="60" y1="7" x2="70" y2="7" />

  <rect x="70" y="2" width="20" height="10" />
  <text x="71" y="8">output</text>
</svg>
```

renders as:

![](simple.svg)


Not too tricky, but there are a lot of fiddly coordinate values, and I've omitted some boilerplate for styling and XML processing instructions. See the underlying file [here](simple.svg).

What if we could render the same thing from something more like the following?

```xml
<svg>
  <rect id="in" xy="10 2" wh="20 10" text="input" />
  <rect id="proc" xy="^h 20" wh="^" text="process" />
  <rect id="out" xy="^h 20" wh="^" text="output" />

  <line start="#in" end="#proc" />
  <line start="#proc" end="#out" />
</svg>
```

This example shows just a few of the enhancements `svgdx` provides:

* shortcut attributes, e.g. the use of `wh` rather than having to specify `width` and `height` separately.
* relative positioning, either by reference to an id (e.g. `#in`), or to the previous element using the caret (`^`) symbol
* new attributes providing additional functionality - `text` within shapes, or `start` and `end` points on `<line>` elements to define connectors.

Many more features are provided by **svgdx**, with the goal of making a diagram something you can write, rather than draw.
