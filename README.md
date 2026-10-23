# dumpany
A stateful, streaming text dumper for structured, width-aware text generation.

## Tests

Run all tests with `cargo test`, or a feature suite with, for example,
`cargo test --test finalization`.

- `tests/formatting.rs`: text, wrapping, indentation, and output fixtures.
- `tests/contexts.rs`: context lifecycle and scoped cleanup.
- `tests/targets.rs`: prologues, epilogues, and target routing.
- `tests/composition.rs`: iterators, joins, and tuples.
- `tests/finalization.rs`: finalization and writer failures.
- `tests/common/mod.rs`: shared writers and fixture helpers.

Add public API tests to the appropriate integration suite. Tests needing private
implementation access belong in `src/<module>/tests.rs`, declared by the parent
module with `#[cfg(test)] mod tests;`. Expected output lives in `test-data/check/`;
fixture tests write actual output to `test-data/result/`.
