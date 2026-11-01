"""Check an installed distribution without running the functional test suite."""

from importlib.metadata import version
from pathlib import Path

import dumpany
from dumpany import dumpany as native


def main():
    installed_version = version("dumpany")
    assert dumpany.__version__ == native.__version__ == installed_version

    package_dir = Path(dumpany.__file__).parent
    for filename in ("__init__.pyi", "py.typed"):
        assert (package_dir / filename).is_file(), f"Missing {filename}"

    print(f"Installed dumpany {installed_version}: native import and typing files OK")


if __name__ == "__main__":
    main()
