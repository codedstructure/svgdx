.PHONY: all serve check docs svgdx-server svgdx mdbook wasm clean

all: svgdx-server svgdx

SVGDX_SERVER := target/release/svgdx-server
SVGDX := target/release/svgdx

SRC_FILES := $(shell find src/ -type f) Cargo.toml Cargo.lock
EDITOR_FILES := $(shell find editor/ -type f)

svgdx-server: $(SVGDX_SERVER)
$(SVGDX_SERVER): $(SRC_FILES) $(EDITOR_FILES)
	cargo build --release --bin svgdx-server --no-default-features --features "server"

svgdx: $(SVGDX)
$(SVGDX): $(SRC_FILES)
	cargo build --release --bin svgdx --no-default-features --features "cli"

serve: svgdx-server
	$(SVGDX_SERVER) --open

check:
	sh scripts/check.sh

docs:
	sh scripts/docs.sh

mdbook: svgdx
	MDBOOK_SVGDX_BIN="$(abspath $(SVGDX))" mdbook serve --open docs/mdbook

wasm:
	sh scripts/wasm_build.sh

clean:
	cargo clean
