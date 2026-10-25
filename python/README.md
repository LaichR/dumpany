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
output; writes are not rolled back.
The module also exposes `dumpany.__version__`.

## Composing dump operations

`dump(value)` accepts newline operations, recursively nested iterables, and
arbitrary objects converted with Python's `str()`:

```python
from dumpany import Join, NewLine

dumper = Dumper(stream)
dumper.dump([
    "Heading", NewLine(increment=1),
    ["Count: ", 42], ...,
    "Done", NewLine(decrement=1),
])
dumper.dump_all()
```

`...` and `NewLine()` emit a newline without changing indentation.
`NewLine(increment=1)` and `NewLine(decrement=1)` change indentation after
emitting the line. Counts are integers from 0 to 255; supplying both keywords
is an error.

Strings and bytes-like objects (`bytes`, `bytearray`, `memoryview`) are atomic:
they use their string representation rather than iterating or implicitly
decoding bytes. Other iterables, including generators, are traversed without
implicit separators; mappings yield their keys. Iteration takes precedence
over `__str__`. Active iterable cycles raise `ValueError`, and nesting beyond
100 iterable levels raises `RecursionError`. Repeated non-cyclic references are
allowed. Conversion and iteration exceptions propagate; preceding output is
not rolled back.

`Join(delimiter, iterable)` dumps items with a delimiter only between items:

```python
dumper.dump(Join(", ", ["red", "green", "blue"]))
dumper.dump(...)
dumper.dump(Join(..., ["first row", "second row"]))
```

Items and delimiters use the same recursive dispatch as `dump()`, so delimiters
can also be operation lists. Empty iterables emit nothing. Iteration starts only
when the operation is dumped; a join holding a generator or other one-shot
iterator is effectively one-shot. Join operations and their iterables participate
in cycle detection and the nesting limit. An explicitly supplied string iterable
is traversed by `Join`, while string and bytes-like items remain atomic.

## Context operations

`Context(content, *, prefix=None, name=None)` controls the indentation and
alignment of a text block independently of the normal indentation level.
Its prefix establishes where the block's content begins; subsequent lines,
including automatically wrapped lines, align with that content rather than
repeating the prefix. This is useful for enumeration items, function parameters,
and similar structural elements.

Give each enumeration item its own context:

```python
import io
from dumpany import Context, Dumper

stream = io.StringIO()
dumper = Dumper(stream, line_width=40)
dumper.dump([
    Context(["Build the packages", ..., "Include Rust and Python."], prefix="1. "),
    Context(["Run the tests", ..., "Check both packages."], prefix="2. "),
])
dumper.dump_all()
print(stream.getvalue(), end="")
```

Output:

```text
1. Build the packages
   Include Rust and Python.
2. Run the tests
   Check both packages.
```

Closing each context ends its block and restores the enclosing alignment.
Content can be any supported value, including a generator or iterable visitor;
it is consumed only when the context is dumped.

Names must be unique within the recorded
root context and become available for reuse after that root is flushed. Names
and prefixes may be dynamically created Python strings.

The context closes even when its content raises; the original content exception
takes precedence if closing also fails. Previously recorded content is not
rolled back. Context operations participate in cycle detection, depth limits,
and Python garbage collection, just like `Join`.

This is an operation object, not a Python `with` context manager. Iterables allow
lazy Python-side content generation, but Rust still buffers each root context's
encoded operations until it closes.

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
