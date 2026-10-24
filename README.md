# dumpany
A stateful, streaming text dumper for structured, width-aware text generation.

## Workspace layout

The root `Cargo.toml` is a virtual workspace with two member packages:

- `rust/`: the `dumpany` Rust library, including its tests and fixtures.
- `python/`: the `dumpany-python` PyO3 binding crate, built with maturin as the
  importable `dumpany` native extension, with no Python implementation
  code. It depends on the Rust library and provides a
  separate Python interface, with its own version and release lifecycle.

The workspace shares a root `Cargo.lock` and `target/` directory. Root-level
Cargo commands target the Rust library by default; use `--workspace` to select
both packages or `-p dumpany-python` to select the bindings.

## Python package (scaffolding)

The API is implemented entirely in Rust. Maturin generates a minimal Python
import shim in the wheel; no Python package source is maintained here. Python
files under `python/tests/` only test the installed extension.

The Python module exposes `dumpany.__version__` and a minimal `dumpany.Dumper`
supporting text output to a Python text stream:

```python
import io
from dumpany import Dumper

stream = io.StringIO()
dumper = Dumper(stream)
dumper.text("hello")
dumper.dump_all()  # Emits buffered text and flushes; does not close the stream.
assert stream.getvalue() == "hello\n"
```

`text()` accepts strings. `dump_all()` finalizes the dumper; subsequent calls
to `text()` fail. Contexts and other dump operations are not exposed yet.

The constructor accepts keyword-only configuration: `Dumper(stream,
line_width=120, new_line="\n")`. For example, use `new_line="\r\n"` for
Windows-style line endings. Indentation retains the Rust default of four spaces.

The Rust-side `PythonWriter` adapter accepts Python text streams such as
`io.StringIO` and `sys.stdout` for use with the Dumper. It forwards writes and
flushes without closing the stream, and preserves Python exceptions in I/O
errors. Each write must contain complete UTF-8.

With Python 3.10+ and Rust installed, from the repository root:

```sh
python -m venv .venv
# Activate .venv using your shell's activation command.
python -m pip install "maturin>=1.9,<2" pytest
cargo test --locked -p dumpany-python
maturin develop --manifest-path python/Cargo.toml
python -m pytest python/tests
maturin build --release --locked --manifest-path python/Cargo.toml --out dist
```

The Python workflow builds and tests CPython stable-ABI wheels for Linux x86-64 and ARM64,
Windows x86-64, and macOS ARM64, plus a source distribution. Wheels support
standard CPython 3.10+ (not free-threaded Python). Build artifacts are uploaded
to GitHub Actions; nothing is published to PyPI.

## Tests

Run all tests with `cargo test`, or a feature suite with, for example,
`cargo test --test finalization`.

- `rust/tests/formatting.rs`: text, wrapping, indentation, and output fixtures.
- `rust/tests/contexts.rs`: context lifecycle and scoped cleanup.
- `rust/tests/targets.rs`: prologues, epilogues, and target routing.
- `rust/tests/composition.rs`: iterators, joins, and tuples.
- `rust/tests/finalization.rs`: finalization and writer failures.
- `rust/tests/common/mod.rs`: shared writers and fixture helpers.

Add public API tests to the appropriate integration suite. Tests needing private
implementation access belong in `rust/src/<module>/tests.rs`, declared by the parent
module with `#[cfg(test)] mod tests;`. Expected output lives in `rust/test-data/check/`;
fixture tests write actual output to `rust/test-data/result/`.
