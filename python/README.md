# dumpany — Python extension

A native Python extension for width-aware text output, implemented entirely in
Rust with PyO3. The Python distribution and import name are `dumpany`; the Rust
binding crate is named `dumpany-python`.

No release has been tagged or published to PyPI yet. See
[CHANGELOG.md](CHANGELOG.md) for the initial feature set and the
[workspace README](../README.md) for development information.

## Example

```python
import io
from dumpany import Dumper

stream = io.StringIO()
dumper = Dumper(stream, line_width=120, new_line="\n")
dumper.text("hello")
dumper.dump_all()
assert stream.getvalue() == "hello\n"
```

`Dumper(stream, *, line_width=120, new_line="\n")` accepts Python text streams such
as `io.StringIO` and `sys.stdout`. Use `new_line="\r\n"` for Windows-style line
endings. Indentation retains the Rust default of four spaces.

`text()` accepts strings and may buffer output. `dump_all()` emits pending text
and flushes without closing the stream. It finalizes the dumper: subsequent calls
to `text()` fail; successful repeated calls to `dump_all()` do nothing. Finalize
explicitly rather than relying on object destruction.

Python stream exceptions are preserved. A failed write may leave incomplete
output; writes are not rolled back. Contexts and other dump operations are not
exposed yet. The module also exposes `dumpany.__version__`.

## Build and test

With Python 3.10+ and Rust installed, from the workspace root:

```sh
python -m venv .venv
# Activate .venv using your shell's activation command.
python -m pip install "maturin>=1.9,<2" pytest
cargo test --locked -p dumpany-python
maturin develop --manifest-path python/Cargo.toml
python -m pytest python/tests
maturin build --release --locked --manifest-path python/Cargo.toml --out dist
```

Maturin generates a minimal Python import shim in the wheel; no Python package
source is maintained here. Files under `tests/` test the installed extension.
Wheels target the CPython stable ABI for Python 3.10+ (not free-threaded Python).

The Rust-side `PythonWriter` adapter forwards text writes and flushes without
closing the stream, retaining Python exceptions in I/O errors. Each Rust write
must contain complete UTF-8.
