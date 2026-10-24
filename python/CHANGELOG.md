# Changelog

## [Unreleased]

### Added

- Native Python extension exposing `Dumper` and package version information.
- Text output to Python text streams, including `io.StringIO` and `sys.stdout`.
- Configurable line width and line endings.
- Explicit finalization without closing the supplied stream.
- Recursive dumping of iterables and string representations of Python objects.
- Newline operations using `...` or `NewLine` with optional indentation changes.
- Detection of iterable cycles and excessive nesting during dumping.
