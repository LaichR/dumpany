# Changelog

## [Unreleased]

## [0.1.0] - 2026-11-14

### Added

- Initial feature set:
  - Text output to Python streams with configurable line width and line endings.
  - Indivisible text through `NoBreak` and `Dumper.no_break()`, with Unicode character-based wrapping and a 65,535-character limit per no-break value.
  - Recursive composition of iterables, Python objects, and dump operations.
  - Line breaks and explicit indentation changes.
  - Nested, aligned contexts with prefixes, prologues, and epilogues.
  - Optional inline context closing with `new_line=False`; indentation changes always end the line.
  - Joining content with delimiters.
  - Explicit output finalization without closing the supplied stream.
  - Python user documentation and type stubs.
