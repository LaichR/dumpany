# dumpany — Rust library

A text-generation library for structured output, with control over line width,
indentation, and alignment.

## Library structure

- `core`: the dumper, configuration, and traits for composing output.
- `context`: aligned text blocks with optional prefixes and names.
- `dump_tuple`: composition of heterogeneous dump operations using tuples.

The internal `operations` and `recorder` modules support recording and rendering
and are not part of the public API.

Implementation and unit tests are in `src/`; public API integration tests are in
`tests/`. Formatting fixtures are in `test-data/`.

## Development

Run these commands from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --locked -p dumpany --all-targets -- -D warnings
cargo test --locked -p dumpany
```

To apply formatting, run `cargo fmt --all` without `-- --check`.

## Documentation

See the [Rust documentation](https://laichr.github.io/dumpany/rust/dumpany/)
for usage, examples, and the API reference.

Build and open the Rust API documentation locally:

```sh
cargo doc --locked -p dumpany --no-deps --open
```

The generated entry page is `target/doc/dumpany/index.html`.
