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
dumper.dump("hello")
dumper.dump_all()
assert stream.getvalue() == "hello\n"
```

`Dumper(stream, *, line_width=120, new_line="\n")` accepts Python text streams such
as `io.StringIO` and `sys.stdout`. Use `new_line="\r\n"` for Windows-style line
endings. Indentation retains the Rust default of four spaces.

`dump()` accepts strings and other objects and may buffer output.
`dump_all()` emits pending text
and flushes without closing the stream. It finalizes the dumper: subsequent calls
to `dump()` that add text fail; successful repeated calls to `dump_all()` do nothing. Finalize
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

`Context(content, *, prefix=None, name=None, increment=None, decrement=None)` controls the indentation and
alignment of a text block independently of the normal indentation level.
Its prefix establishes where the block's content begins; subsequent lines,
including automatically wrapped lines, align with that content rather than
repeating the prefix. This is useful for enumeration items, function parameters,
and similar structural elements.

Closing each context ends its block and restores the enclosing alignment.
Normal indentation is unchanged unless `increment` or `decrement` is supplied;
that change applies after the final line, without an extra newline. Supply only
one of these keywords, with a count from 0 to 255. Indentation changes inside
the context are not automatically undone when it closes.
Content can be any supported value, including a generator or iterable visitor;
it is consumed only when the context is dumped.

Names must be unique within the outermost context and become available for
reuse after that context is flushed. Names
and prefixes may be dynamically created Python strings.

The context closes even when its content raises; the original content exception
takes precedence if closing also fails. Previously recorded content is not
rolled back. Context operations participate in cycle detection, depth limits,
and Python garbage collection, just like `Join`.

This is an operation object, not a Python `with` context manager. Iterables allow
lazy Python-side content generation, but Rust still buffers each outermost context's
encoded operations until it closes.

## Prologues and epilogues

`Prologue(name, content)` and `Epilogue(name, content)` record content before or
after the body of a named context, regardless of where the operations occur in
the traversal. The example below uses this to place a count before the items
being counted.

The named context must already be registered within an open outermost context.
Closed child contexts remain targetable until the outermost context is rendered. Repeated operations
append to the same prologue or epilogue. Target content uses ordinary recursive
dispatch, including lazy visitors, joins, newlines, and context operations.

These operations change the recording destination, not alignment or context
structure. Selecting another prologue or epilogue from within target content is
not allowed. Main recording is restored even if content raises; the original
exception propagates and recorded content is not rolled back. Target operations
participate in cycle detection, nesting limits, and Python garbage collection.

## Putting it all together

This example turns a Python object into a YAML mapping with a class name, a pupil
count, and a list of pupils. The pupils can come from a generator: the count is
calculated while producing the list, then inserted above it using a prologue.

```python
import io
import json
from collections.abc import Iterable
from dataclasses import dataclass

from dumpany import Context, Dumper, NewLine, Prologue


@dataclass
class Pupil:
    name: str
    surname: str


@dataclass
class SchoolClass:
    name: str
    pupils: Iterable[Pupil]


def class_yaml(school_class):
    count = 0
    yield NewLine(increment=1)
    for pupil in school_class.pupils:
        count += 1
        yield Context([
            "name: ", json.dumps(pupil.name), ...,
            "surname: ", json.dumps(pupil.surname),
        ], prefix="- ")
    yield NewLine(decrement=1)
    yield Prologue("school-class", [
        "name: ", json.dumps(school_class.name), ...,
        "pupil_count: ", count, ...,
        "pupils:", " []" if count == 0 else "",
    ])


school_class = SchoolClass("Class 4B", (
    pupil for pupil in [Pupil("Alice", "Smith"), Pupil("Ben", "Jones")]
))
stream = io.StringIO()
dumper = Dumper(stream)
dumper.dump(Context(class_yaml(school_class), name="school-class"))
dumper.dump_all()
print(stream.getvalue(), end="")
```

Output (apart from trailing blank lines):

```yaml
name: "Class 4B"
pupil_count: 2
pupils:
    - name: "Alice"
      surname: "Smith"
    - name: "Ben"
      surname: "Jones"
```

- `NewLine(increment=1)` indents the list by four spaces; the matching decrement
  restores the previous indentation.
- Each `Context(prefix="- ")` introduces a list item and aligns `surname` with
  `name`, without repeating the dash.
- `class_yaml()` does not know the pupil count in advance. It counts pupils as
  it visits them, then uses `Prologue` to place the completed count above the
  list. The surrounding named context stays open throughout that traversal.
- `json.dumps()` quotes the names using YAML-compatible double-quoted strings.
  An empty iterable produces `pupil_count: 0` and `pupils: []`.

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

### Type stubs

Public classes, constructors, and methods carry documentation and
`pyo3-stub-gen` annotations in the Rust bindings. Regenerate `dumpany.pyi` after
changing the Python interface:

```sh
cargo run --locked -p dumpany-python --bin generate_type_stubs
```

The Python shared library must be available on the library search path when
running the generator (on Windows, add the Python installation directory to
`PATH`). Internal garbage-collection hooks are intentionally omitted.

The Rust-side `PythonWriter` adapter forwards text writes and flushes without
closing the stream, retaining Python exceptions in I/O errors. Each Rust write
must contain complete UTF-8.
