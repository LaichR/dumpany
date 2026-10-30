import importlib.metadata
from importlib.machinery import EXTENSION_SUFFIXES
from pathlib import Path

import dumpany


def test_native_extension_imports():
    from dumpany import dumpany as native

    assert any(native.__file__.endswith(suffix) for suffix in EXTENSION_SUFFIXES)
    assert dumpany.__version__ == native.__version__


def test_package_version_matches_metadata():
    assert dumpany.__version__ == importlib.metadata.version("dumpany")


def test_package_includes_type_stubs():
    package_dir = Path(dumpany.__file__).parent
    assert (package_dir / "__init__.pyi").is_file()
    assert (package_dir / "py.typed").is_file()
