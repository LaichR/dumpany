# dumpany — Rust library

A streaming text dumper with configurable line width, indentation, and line
endings. Structured contexts support prefixes, prologues, and epilogues;
the `Dump` trait, iterators, joins, and tuples support composition.

No release has been tagged yet. See [CHANGELOG.md](CHANGELOG.md) for the initial
feature set and the [workspace README](../README.md) for development information.

## Example: generate a Markdown report

Contexts apply a list prefix and indent wrapped continuation lines for you.
`join()` inserts table-column separators without special-casing the last column.

```rust
use dumpany::context::Context;
use dumpany::core::{Dumper, DumperConfig, Text};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = Dumper::new(
        Vec::new(),
        DumperConfig {
            line_width: 48,
            ..DumperConfig::default()
        },
    );
    d.text("# Release checklist\n\n")?;

    for task in [
        "Run the Rust and Python tests before publishing the release.",
        "Build the wheels and review the package documentation.",
    ] {
        Context::new(Some("- "), None).with(&mut d, |d| d.text(task))?;
    }

    d.text("\n## Packages\n\n")?;
    for row in [
        ["Package", "Status"],
        ["---", "---"],
        ["Rust crate", "Ready"],
        ["Python extension", "Preview"],
    ] {
        Context::new(Some("| "), None).with(&mut d, |d| {
            d.join(row.map(Text), Text(" | "))?;
            d.text(" |")
        })?;
    }

    print!("{}", String::from_utf8(d.into_inner()?)?);
    Ok(())
}
```

Output:

```text
# Release checklist

- Run the Rust and Python tests before
  publishing the release.
- Build the wheels and review the package
  documentation.

## Packages

| Package | Status |
| --- | --- |
| Rust crate | Ready |
| Python extension | Preview |
```

Run the [complete example](examples/markdown.rs) from the workspace root:

```sh
cargo run -p dumpany --example markdown
```

Table cells are kept short here: `join()` inserts separators, but does not
calculate column widths or escape Markdown content.

`DumperConfig` defaults to a line width of 120, four-space indentation, and
`"\n"` line endings.

## Writer ownership and finalization

`Dumper::new(writer, config)` takes its writer directly. Pass an owned writer or
`&mut writer` to borrow one. `get_ref()` borrows the writer without flushing
buffered text. The dumper is `Send` when its writer is `Send`.

`dump_all()` closes pending contexts, emits pending text, and flushes the writer.
It finalizes the dumper: subsequent content operations fail, while successful
repeated finalization does nothing. Dropping a dumper does not finalize it.

`into_inner()` consumes the dumper, finalizes pending output, and returns the
underlying writer. Already-finalized dumps are not flushed again.

If finalization fails, remaining buffered output is discarded:

- `IntoInnerError::error()` borrows the original I/O error.
- The error's `into_inner()` returns the writer as-is.
- The error's `into_parts()` returns both the original error and writer.

Failed writes are not retried or rolled back. Output may be incomplete, including
a partial line or UTF-8 sequence; returning the writer does not repair its content.

## Tests

From the workspace root:

```sh
cargo test --locked -p dumpany
cargo test --locked -p dumpany --test finalization
```

- `tests/formatting.rs`: text, wrapping, indentation, and output fixtures.
- `tests/contexts.rs`: context lifecycle and scoped cleanup.
- `tests/targets.rs`: prologues, epilogues, and target routing.
- `tests/composition.rs`: iterators, joins, and tuples.
- `tests/finalization.rs`: finalization, writer ownership, and failures.
- `tests/common/mod.rs`: test writers and fixture helpers.

Add public API tests to the appropriate integration suite. Tests needing private
implementation access belong in `src/<module>/tests.rs`, declared by the parent
module with `#[cfg(test)] mod tests;`. Expected output lives in `test-data/check/`;
fixture tests write actual output to `test-data/result/`.
