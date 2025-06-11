.PHONY: serve check docs wasm

SVGDX_SERVER=target/debug/svgdx-server

$(SVGDX_SERVER):
	cargo build --bin svgdx-server

serve: $(SVGDX_SERVER)
	cargo run --bin svgdx-server -- --open

check:
	sh scripts/check.sh

docs:
	sh scripts/docs.sh

wasm:
	sh scripts/wasm_build.sh
