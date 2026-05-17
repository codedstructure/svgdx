.PHONY: serve check docs svgdx mdbook wasm

SVGDX_SERVER=target/release/svgdx-server
SVGDX=target/release/svgdx

$(SVGDX_SERVER):
	cargo build --release --bin svgdx-server

$(SVGDX):
	cargo build --release --bin svgdx

serve: $(SVGDX_SERVER)
	cargo run --release --bin svgdx-server -- --open

check:
	sh scripts/check.sh

docs:
	sh scripts/docs.sh

svgdx: $(SVGDX)

mdbook: $(SVGDX)
	MDBOOK_SVGDX_BIN="$(abspath $(SVGDX))" mdbook serve --open docs/mdbook

wasm:
	sh scripts/wasm_build.sh
