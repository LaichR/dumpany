# Building the Python documentation

The documentation uses Sphinx, Furo, and AutoAPI with custom templates and CSS.
AutoAPI reads `python/dumpany.pyi`; it does not import the native extension.

Build the documentation from the repository root, with your virtual environment activated:

```sh
python -m pip install -r python/docs/requirements.txt
cargo run --locked -p dumpany-python --bin generate_type_stubs
python -m sphinx -W --keep-going -b html python/docs python/docs/_build/html
```

Open `python/docs/_build/html/index.html` to view the result.

## Publishing

Documentation is published by CI.
