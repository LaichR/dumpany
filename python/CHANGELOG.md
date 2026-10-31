# Changelog

## [Unreleased]

### Added

- Optional indentation increments and decrements when closing a context,
  without an additional line break.

- Prologue and epilogue operations for adding content before or after named
  context bodies while traversing their content.

- Context operations with optional runtime prefixes and names, nested content,
  and lazy iterable traversal.

- Native Python extension exposing `Dumper` and package version information.
- Text output to Python text streams, including `io.StringIO` and `sys.stdout`.
- Configurable line width and line endings.
- Explicit finalization without closing the supplied stream.
- Recursive dumping of iterables and string representations of Python objects.
- Newline operations using `...` or `NewLine` with optional indentation changes.
- Lazy `Join` operations with recursively dispatched items and delimiters.
- Detection of iterable cycles and excessive nesting during dumping.
