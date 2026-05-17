# mdbook docs for svgdx

This directory contains the documentation for `svgdx` in mdbook format.

At present it is a work-in-progress...

## The mdbook-svgdx preprocessor

As well as [mdbook](https://rust-lang.github.io/mdBook/) itself, these docs use the [mdbook-svgdx](https://github.com/codedstructure/mdbook-svgdx) preprocessor to render embedded `svgdx` fragments.

The preprocessor supports an `MDBOOK_SVGDX_BIN` environment variable pointing at the `svgdx` CLI binary to use for rendering embedded diagrams.
That allows the book to be built against the current checkout's release binary, rather than the `svgdx` library version bundled into the installed `mdbook-svgdx` crate.

## Building and updating the docs

Install mdbook with `cargo install mdbook`, and install the `mdbook-svgdx` pre-processor without its default bundled `svgdx` dependency:

```shell
cargo install mdbook-svgdx --no-default-features
```

From the repository root, build the current release `svgdx` binary and launch mdBook with `MDBOOK_SVGDX_BIN` pointing at `target/release/svgdx`.

Build the documentation using:

```shell
cargo build --release --bin svgdx
MDBOOK_SVGDX_BIN="$PWD/target/release/svgdx" mdbook build docs/mdbook
```

When editing the documentation, the following is often more helpful:

```shell
make mdbook
```

See the [mdbook documentation](https://rust-lang.github.io/mdBook/) for more information.
