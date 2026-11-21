# Changelog

## [Unreleased]

## [0.1.0] - 2026-11-14

### Added

- Initial feature set:
  - Text output with configurable line width, line endings, and indentation.
  - Indivisible text through `NoBreak` and `Dumper::no_break`, with Unicode character-based wrapping and a 65,535-character limit per no-break value.
  - Nested, aligned contexts with prefixes, prologues, and epilogues.
  - Composition through iterators, joins, tuples, and the `Dump` trait.
  - Explicit output finalization.
