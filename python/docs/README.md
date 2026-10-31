# Building the Python documentation

The documentation uses Sphinx, Furo, and AutoAPI with custom templates and CSS.
AutoAPI reads `python/dumpany.pyi`; it does not import the native extension.

From the repository root, with your virtual environment activated:

```sh
python -m pip install -r python/docs/requirements.txt
cargo run --locked -p dumpany-python --bin generate_type_stubs
python -m sphinx -W --keep-going -b html python/docs python/docs/_build/html
```

The stub generator needs a Python runtime. On Windows, add the Python
installation directory to `PATH` if its shared library cannot be found.
Regenerate the stub after changing the Python interface.

Open `python/docs/_build/html/index.html` to view the result.

## Publishing

The Documentation workflow builds pull requests targeting `main`, pushes to
`main`, and tags. It publishes only on a tag push when the repository is public.
Private-repository tags and branch pushes never publish documentation.

Before the first public release, select **GitHub Actions** as the source under
**Settings → Pages → Build and deployment** and allow tag deployments in the
`github-pages` environment.

Python documentation is published under `python/`, alongside a root landing
page. Future Rust documentation can share the same Pages artifact. Each
deployment replaces the complete site; tags do not create versioned directories.

## Customization

- `conf.py`: AutoAPI settings, Furo options, and template filters.
- `_templates/autoapi/`: API page templates.
- `_static/css/doc_styles.css`: custom styling.
- `index.rst`, `getting-started.rst`, and `api.rst`: documentation entry pages.
