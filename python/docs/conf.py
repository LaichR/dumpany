"""Sphinx configuration for the dumpany Python documentation."""
import re
from pathlib import Path

from importlib.metadata import PackageNotFoundError, version as package_version

project = "dumpany"
DOCS_DIR = Path(__file__).resolve().parent

try:
    release = package_version("dumpany")
except PackageNotFoundError:
    release = "unreleased"

# Add any Sphinx extension module names here, as strings. They can be
# extensions coming with Sphinx (named 'sphinx.ext.*') or your custom
# ones.
extensions = [
    'autoapi.extension',
    'sphinx_tabs.tabs',
    'sphinx.ext.intersphinx',
]



# List of patterns, relative to source directory, that match files and
# directories to ignore when looking for source files.
# This pattern also affects html_static_path and html_extra_path.
exclude_patterns = [
    '_build', 'Thumbs.db', '.DS_Store', '_templates', 'README.md',
]
templates_path = ['_templates']
add_module_names = False
autoapi_type = 'python'
autoapi_root = 'autoapi'
autoapi_add_toctree_entry = False
autoapi_template_dir = "_templates/autoapi"
autoapi_dirs = [str(DOCS_DIR.parent)]
autoapi_file_patterns = ['dumpany.pyi']
autoapi_ignore = []
autoapi_options = ['members', 'undoc-members', 'special-members', 'show-module-summary']
autoapi_python_class_content = 'both'
autodoc_typehints = "signature"
autodoc_typehints_format = "short"
autoapi_own_page_level = "class"

intersphinx_mapping = {
    "python": ("https://docs.python.org/3/", None),
}

# -- Options for HTML output -------------------------------------------------

# The theme to use for HTML and HTML Help pages.  See the documentation for
# a list of builtin themes.
#
html_theme = 'furo'
html_theme_options = {
    'navigation_with_keys': True,
    'sidebar_hide_name': False,
}
html_css_files = ['css/doc_styles.css']
html_static_path = ['_static']
def autoapi_prepare_jinja_env(jinja_env):
    # Matches dotted identifiers, including inside roles/backticks/typing text.
    _PATH_EXPRESSION = re.compile(r"(?<!~)\b(?:[A-Za-z_]\w*\.)+([A-Za-z_]\w*)\b")

    def summary_link_style(content):
        return f"\n.. rst-class:: summary-link\n\n{content}"

    def base_references(bases: list) -> str:
        label = "**Bases:**"
        content = ", ".join([f":class:`{name.split('.')[-1]} <{name}>`" for name in bases])
        return f"\n.. rst-class:: bases\n\n{label} {content}"

    def shorten_type_expr(s: str) -> str:
        if not s:
            return s

        # 1) Shorten qualified names
        s = _PATH_EXPRESSION.sub(r"\1", s)
        _OPTIONAL = re.compile(r"\bOptional\[(.+?)\]")

        # 2) Convert Optional[T] -> T | None
        # Repeat until no more Optional[...] patterns exist
        while True:
            new = _OPTIONAL.sub(r"\1 | None", s)
            if new == s:
                break
            s = new

        return s


    _QUALNAME = re.compile(r"\b(?:[A-Za-z_]\w*\.)+([A-Za-z_]\w*)\b")

    def shorten_qualnames(s: str) -> str:
        # pkg.mod.Type -> Type   (also inside list[pkg.Type], Callable[[pkg.A], pkg.B], etc.)
        return _QUALNAME.sub(r"\1", s)

    def module_from_fully_qualified_name(fully_qualified_name: str) -> str:
        index = fully_qualified_name.rfind('.')
        if index == -1:
            return fully_qualified_name
        return fully_qualified_name[:index]

    def cls_docstring(cls):
        """Use the generated stub's documentation without importing the extension."""
        return cls.docstring

    def filter_properties(property: str)->str:
        property_dict = {'abstractmethod': 'abstract'}
        return property_dict.get(property, property)

    jinja_env.filters["shorten_qualnames"] = shorten_qualnames

    jinja_env.filters["shorten_type_expr"] = shorten_type_expr

    jinja_env.filters["cls_docstring"] = cls_docstring

    jinja_env.filters["base_references"] = base_references

    jinja_env.filters["filter_properties"] = filter_properties

    jinja_env.filters["summary_link_style"] = summary_link_style

    jinja_env.filters["module_from_fully_qualified_name"] = module_from_fully_qualified_name

    jinja_env.tests["contains"] = lambda sequence, item: item in sequence

# skip imports and inherited classes; base classes are still displayed
def skip_member(app, what, name, obj, skip, options):
    if obj is not None:
        if obj.imported or obj.inherited:
            return True
    return skip


def setup(app):
    app.connect("autoapi-skip-member", skip_member)
