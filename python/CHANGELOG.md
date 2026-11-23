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
  - `NoBreakRegion` for composed output and iterators, continuing at the current position without automatic wrapping and preserving explicit line breaks.
  - Joining content with delimiters.
  - Explicit output finalization without closing the supplied stream.
  - Optional finalization without a trailing newline through `dump_all(new_line=False)`.
  - Python user documentation and type stubs.
