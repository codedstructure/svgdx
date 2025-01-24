# mdbook docs for svgdx

This directory contains the documentation for `svgdx` in mdbook format.

## The mdbook-svgdx preprocessor

As well as [mdbook](https://rust-lang.github.io/mdBook/) itself, these docs use the [mdbook-svgdx](https://github.com/codedstructure/mdbook-svgdx) preprocessor to render embedded `svgdx` fragments.

Embedded svgdx fragments in these docs assume the latest version of svgdx (i.e. the `main` branch of this repo), and may not build correctly after a simple `cargo install mdbook-svgdx`.
To build against the latest svgdx, the mdbook-svgdx
repository should be cloned and built (`cargo build --release`) after making a change analogous to the following in its Cargo.toml:

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -16,7 +16,7 @@ pulldown-cmark = "0.10"
 pulldown-cmark-to-cmark = "14.0"
 semver = "1.0"
 serde_json = "1.0"
-svgdx = { version = "0.16.0", default-features = false }
+svgdx = { path = "../svgdx", default-features = false }

 [dev-dependencies]
 assertables = "9.5.0"
```

(assuming that `mdbook-svgdx` and `svgdx` repositories are both checked out at the same level within the filesystem.)

The generated `mdbook-svgdx` executable should be available in the system PATH.

## Building and updating the docs

Install mdbook with `cargo install mdbook`, and the `mdbook-svgdx` pre-processor as in the previous section.

Build the documentation using:

```shell
mdbook build
```

When editing the documentation, the following is often more helpful:

```shell
mdbook serve --open
```

See the [mdbook documentation](https://rust-lang.github.io/mdBook/) for more information.
