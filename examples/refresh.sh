#/usr/bin/env bash

for SVG_IN in *-in.svg; do
    SVG_OUT="${SVG_IN/-in.svg/-out.svg}"
    cargo run --release -q -- "$SVG_IN" -o "$SVG_OUT"
done
