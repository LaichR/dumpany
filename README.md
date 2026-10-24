# dumpany

A stateful, streaming text dumper for structured, width-aware text generation.

## Packages

- [Rust crate](rust/README.md): the `dumpany` library, with structured contexts,
  formatting, and composable dump operations. [Changelog](rust/CHANGELOG.md).
- [Python extension](python/README.md): the `dumpany` native extension, implemented
  in Rust with PyO3. [Changelog](python/CHANGELOG.md).

The packages have independent versions and release cycles. No releases have been
tagged yet; each package's changelog summarizes its initial feature set under
`Unreleased`, rather than listing intermediate development changes.

## Workspace development

The root `Cargo.toml` is a virtual workspace containing `rust/` and `python/`.
They share a root `Cargo.lock` and `target/` directory. The Python binding crate
is named `dumpany-python`; its Python distribution and import name are `dumpany`.

Root-level Cargo commands target the Rust library by default. Use `--workspace`
to select both packages or `-p dumpany-python` to select the bindings.

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked
```

See the package READMEs for API examples, package-specific tests, and Python
build instructions.

## Continuous integration

The Rust workflow checks formatting, runs Clippy, and tests the library.
The Python workflow builds and tests stable-ABI wheels for Linux x86-64 and
ARM64, Windows x86-64, and macOS ARM64, plus a source distribution. Artifacts
are uploaded to GitHub Actions; nothing is published to PyPI.
