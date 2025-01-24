# Introduction

```svgdx
<svg>
  <style>text {font-weight: bold; font-size:4px}</style>
  <rect surround="#input #svgdx #output" margin="10" class="d-fill-darkgrey"/>
  <rect surround="#input #svgdx" margin="7 4" opacity="0.5" class="d-dash d-fill-cornflowerblue d-text-small" text="svgdx" text-loc="tl"/>
  <rect surround="#svgdx #output" margin="4 4" opacity="0.5" class="d-dot d-fill-green d-text-small" text="svg" text-loc="tr"/>

  <rect id="svgdx" wh="30 10" text="svgdx" class="d-fill-teal d-text-ol"/>
  <rect id="input" xy="#svgdx|H 10" wh="30 10" text="input.xml" rx="3" class="d-fill-cornflowerblue d-text-ol"/>
  <rect id="output" xy="#svgdx|h 10" wh="30 10" text="diagram.svg" rx="3" class="d-fill-green d-text-ol"/>

  <line start="#input" end="#svgdx" class="d-arrow"/>
  <line start="#svgdx" end="#output" class="d-arrow"/>
</svg>
```

## The Goal

In my computing life, my happy place is the terminal. I _enjoy_ typing - pressing each of those little buttons on the keyboard in front of me, the steady 'duh-duh-duh-duh' as I type, the appearance of glyphs on the screen in response. I might use a keyboard and read from a monitor, but it can feel my mind is connected to the machine.

Typing is about words, and words are about language. They convey feelings and ideas, but though I love words, sometimes they are the wrong tool. If I need to convey or absorb information both accurately and quickly, I turn to a different tool - **diagrams**.

But I don't want to leave my happy place of pressing those oh-so-satisfying buttons in front of me and seeing things appear in response. Maybe I can keep that and still create diagrams? Maybe rather than _drawing_ diagrams, I can _type_ them?

## The Motivation

### Direct manipulation

Let's start with the negatives. Most diagrams are created using the [direct manipulation](https://en.wikipedia.org/wiki/Direct_manipulation_interface) paradigm - a mouse cursor is used to select a rectangle tool, then drag and resize a rectangle at the appropriate place within some canvas. Graphical objects are 'directly manipulated' as they appear, as though the diagramming tool provides virtual shapes and lines which can be conjured out of the air, then squashed, pushed, tweaked, duplicated and destroyed until the diagram satisfies its creator. There will be hundreds of tiny actions - mostly unconscious - which contribute to the final diagram, as the diagrammer gets the final output 'just right'. The history of how the diagram is created is incidental, and only becomes conscious when mistakes are recognised and the magical 'undo' spell is invoked.

Is there an alternative to this WYSIWYG tweak-it-until-you-make-it approach? And is there anything wrong with it anyway?

To answer the second point first, no - there's nothing wrong with it. But it does have strengths and weaknesses, and considering the alternatives will help us see how sometimes other approaches may be better.

### "You probably don't need a static site generator"

* pandoc + Makefile; raw HTML; ...

### SVG

* just write raw SVG.

### HTMX

### C++ & Cppfront

[Cppfront](https://hsutter.github.io/cppfront/)
* same semantics as C++, new syntax.


## Comparison with alternatives

```svgdx
<svg>
  <g id="others">
    <rect wh="30 10" text="idea" class="d-fill-black"/>
    <rect wh="30 3" xy="^|v" text="thinking" class="d-text-italic d-text-gold d-fill-dimgrey d-text-small"/>
    <rect wh="30 10" xy="^|v" text="most DaC\nlanguages" class="d-fill-darkgrey d-text-ol"/>
    <rect wh="30 20" xy="^|v" text="automation\nmagic" class="d-text-italic d-text-red d-fill-silver"/>
    <rect wh="30 10" xy="^|v" text="SVG output" class="d-fill-whitesmoke"/>
  </g>
  <text xy="^@b">Most DaC languages\nlive close to the domain</text>

  <g transform="translate(40)">
    <rect wh="30 10" text="idea" class="d-fill-black"/>
    <rect wh="30 20" xy="^|v" text="thinking,\npencil &amp; paper,\ntrial-and-error" class="d-text-italic d-text-gold d-fill-dimgrey"/>
    <rect wh="30 10" xy="^|v" text="svgdx" class="d-text-ol d-text-white d-text-bold d-fill-darkgrey"/>
    <rect wh="30 3" xy="^|v" text="automation" class="d-text-italic d-text-red d-fill-silver d-text-small"/>
    <rect wh="30 10" xy="^|v" text="SVG output" class="d-fill-whitesmoke"/>
  </g>
  <text xy="^@b">svgdx provides\nmore control</text>
</svg>
```
