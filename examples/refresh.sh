#/usr/bin/env bash

ROOT=$(dirname $(cargo locate-project --message-format plain))

cargo build --release --bin svgdx --no-default-features --features cli

echo -n "Updating examples"
for INPUT in *.xml; do
    SVG_OUT="${INPUT/.xml/.svg}"
    ${ROOT}/target/release/svgdx -i "$INPUT" -o "$SVG_OUT" || echo "Failed to render ${INPUT}"
    echo -n "."
done
echo
