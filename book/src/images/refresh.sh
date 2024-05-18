#/usr/bin/env bash

echo -n "Updating examples"
for INPUT in *.xml; do
    SVG_OUT="${INPUT/.xml/.svg}"
    cargo run --release -q -- "$INPUT" -o "$SVG_OUT" || echo "Failed to render ${INPUT}"
    echo -n "."
done
echo
