import importlib.metadata
from importlib.machinery import EXTENSION_SUFFIXES

import dumpany


def test_native_extension_imports():
    from dumpany import dumpany as native

    assert any(native.__file__.endswith(suffix) for suffix in EXTENSION_SUFFIXES)
    assert dumpany.__version__ == native.__version__


def test_package_version_matches_metadata():
    assert dumpany.__version__ == importlib.metadata.version("dumpany")
