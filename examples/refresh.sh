#/usr/bin/env bash

for INPUT in *.xml; do
    SVG_OUT="${INPUT/.xml/.svg}"
    cargo run --release -q -- "$INPUT" -o "$SVG_OUT" || echo "Failed to render ${INPUT}"
done
