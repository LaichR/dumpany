# Main design goals

- Generate structured text incrementally through `std::io::Write`, without building a complete document tree.
- Keep visitor-driven traversal independent of the Dumper's formatting state.
- Support configurable line width, indentation, and nested contexts with continuation alignment.
- Preserve embedded newlines and existing indentation in arbitrary text.
- Compose content lazily through sequences, joins, and deferred content slots.
- Buffer only what formatting decisions and unresolved content require.
- Provide a reusable Rust core with a natural Python generator interface through optional PyO3 bindings.
