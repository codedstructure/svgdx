# Delta 10 - Library inclusion

> `svgdx` documents can include fragments from library files

## Using libraries

Both the `svgdx` and `svgdx-server` command line programs support library inclusion through the `--include` flag.
This can be given multiple times to include more than one library.

```svgdx
<svg>
  <rect wh="70 15" text-loc="tl" class="d-text-pre" text='<svg>
  <reuse href="#common:c1"/>
</svg>'/>
  <text rel="^" text-loc="br" text="input.xml" class="d-text-small"/>
</svg>
```

```svgdx
<svg>
 <rect wh="70 20" text-loc="tl" class="d-text-pre" text='<svg name="common">
 <specs>
  <circle id="c1" text="dx" r="2"/>
 </specs>
</svg>'/>
 <text rel="^" text-loc="br" text="library.xml" class="d-text-small"/>
</svg>
```

```bash
$ svgdx --include library.xml -i input.xml -o output.svg
```

```svgdx
<svg>
 <rect wh="110 20" text-loc="tl" class="d-text-pre" text='<svg version="1.1" xmlns="http://www.w3.org/2000/svg" ...>
  <style>...</style>
  <circle r="2" class="c1"/>
  <text x="0" y="0" class="d-text c1">dx</text>
</svg>'/>
 <text rel="^" text-loc="br" text="output.svg" class="d-text-small"/>
</svg>
```

Note the example library has a `name="common"` attribute in the root SVG element. The library filename is irrelevant.
If multiple library files are included that share the same name, only the last one will be used.

When reference an element from a library the `href` attribute uses the target id prefixed by the library name and a colon,
for example `href="#common:c1"` in the input file above references the element with id `c1` in the library named `common`.

## Writing library files

A library file suitable for using in svgdx is a normal svgdx file with the following requirements:

* The root `<svg>` element must have a `name` attribute; this is used as the first part of `href` attributes linking to the target element.

* Only `<defs>` and `<specs>` are considered when referencing elements from the library:

  * `<defs>` elements containing a referenced target are included as-is in the output document;
    `<defs>` elements that do not contain any referenced elements are not included in the output.
    This ensures that `<use>` elements in the rendered output have a target within the output.
  * `<specs>` elements are excluded as elsewhere in `svgdx` - they are typically to contain
    `<reuse>` targets that are rendered into the output.

Elements other than `<defs>` and `<specs>` may still be useful so that library files can be
"self-documenting", i.e. include examples of the fragments they include.

Example library document `library.xml`:

```xml-svgdx
<svg name="lib">
 <specs>
   <circle id="c1" text="dx" r="2"
    style="fill:black;" text-style="fill:white"/>
 </specs>
 <reuse href="#c1"/>
</svg>
```
