# Introduction

Markdown is a text based format which allows simple text files to carry both semantic and simple style information. Being able to do both of those in a single workflow - just by _typing_ - allows a flow which would otherwise not be achievable.

Can the same be done for _drawing_ - specifically for **diagrams**? That's what this project aims to deliver.

Text files provide a number of advantages:
* Clear compatibility with version control - meaningful diffs
* Self-describing - text files are easy to (at least approximately) reverse engineer even in the absence of a clear spec.
* Application independence - additional tools can be written to deal with the format
* Easy editing - at least for simple changes

There are several existing tools which convey diagrammatic information in textual form:

* [Mermaid](https://mermaid.js.org)
* [Graphviz](https://graphviz.org)
* [PlantUML](https://plantuml.com)

In most of these, the tools are specialised for various particular forms of data; they apply a degree of intelligence to the semantic content of the text and render diagrams representing this. For the special cases they work great, however adding additional layers of graphical structure beyond that imposed by the tool ends up fighting against it.

> _[Ditaa](https://ditaa.sourceforge.net) is different in that a diagram-structure is already provided as input, and it effectively just renders it. The work of creating the diagram has already been done._

When abstraction fails, moving to a lower layer is often the answer. Rather than starting with the input information, can we work backwards from the end result we want?

Enter **[SVG](https://en.wikipedia.org/wiki/SVG)** - an XML-based language for defining vector images. The tools discussed above (with the exception of `ditaa`) can all output SVG images, and SVG is the lowest-common-denominator of graphics formats for diagramming tools.

SVG is a fairly simple language, and a small subset can be used to create a large number of diagrams:

* Rectangles
* Lines
* Text

Out of these, text is the most tricky, but it's still straightforward to make simple diagrams.

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

What if we could render the same thing something more like the following?

```xml
<svg>
  <def id="smallrect" width="20" height="10" y="2" />

  <tbox id="inp" x="10" from="#smallrect">input</tbox>
  <line start="#inp%r" end="#proc%l" />
  <tbox id="proc" x="40" from="#smallrect">process</tbox>
  <line start="#proc%r" end="#out%l" />
  <tbox id="out" x="70" from="#smallrect">output</tbox>
</svg>
```

This potential example shows a few possible features for an SVG pre-processor:

* the use of attribute injection from a particular source id
* the consistent use of #{id} to reference an element with a given `id` attribute (which, per XML, should be unique in the document)
* a consistent 'mini-language' for referencing particular aspects of other elements; e.g. `#{id}%{direction}`, where `direction` is one of `t` (top), `b` (bottom), `l` (left), `r` (right), `c` (centre), or combinations of `t` / `b` followed by `l` or `r` (e.g. `bl` - bottom-left)
* redefining attributes of existing SVG elements - e.g. providing `start` and `end` attributes for the `<line>` element.
* defining new elements, such as `<tbox>` which will transform into multiple SVG elements
* supports all existing SVG elements and attributes, and will leave these untransformed

A further enhancement may be moving beyond XML to a simple text format:

```
tbox "input"
tbox "process"
tbox "output"
```
