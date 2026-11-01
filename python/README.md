# dumpany — Python extension

Python bindings for the dumpany text-generation library, implemented in
Rust with PyO3.

## Package structure

- `src/`: Python bindings and stream adapter.
- `tests/`: tests of the installed extension.
- `tools/`: type-stub generator.
- `docs/`: Sphinx documentation.
- `dumpany.pyi`: generated Python type stubs.

The Python package and import name are `dumpany`; the Rust binding crate
is named `dumpany-python`.

## Development

With Python 3.10+, Rust, and an activated virtual environment, run these
commands from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --locked -p dumpany-python --all-targets -- -D warnings
cargo test --locked -p dumpany-python
cargo run --locked -p dumpany-python --bin generate_type_stubs
python -m pip install -e "./python[test]"
python -m pytest python/tests
```

If you are in the `python/` directory instead, the Cargo commands remain the
same, but installation and testing use:

```sh
python -m pip install -e ".[test]"
python -m pytest tests
```

After changing the bindings, regenerate the type stubs and repeat the editable
install to rebuild the native extension.
On Windows, the stub generator needs the Python installation directory
on `PATH`.

## Documentation

See the [Python documentation](https://laichr.github.io/dumpany/python/)
for installation, usage, examples, and the API reference.

Documentation build instructions are in [docs/README.md](docs/README.md).
