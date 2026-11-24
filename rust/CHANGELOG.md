# Changelog

## [Unreleased]

## [0.1.0] - 2026-11-14

### Added

- Initial feature set:
  - Text output with configurable line width, line endings, and indentation.
  - Indivisible text through `NoBreak` and `Dumper::no_break`, with Unicode character-based wrapping and a 65,535-character limit per no-break value.
  - Nested, aligned contexts with prefixes, prologues, and epilogues.
  - Optional inline context closing; indentation changes always end the line.
  - Nested no-break regions that continue at the current position, suppressing automatic wrapping while preserving explicit line breaks.
  - Composition through iterators, joins, tuples, and the `Dump` trait.
  - Configurable joins with `Inherit`, `Always`, and `AsNeeded` break policies.
  - Explicit output finalization.
  - Optional finalization without a trailing newline for inline text fragments.
