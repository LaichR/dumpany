"""Validate a release tag against its package's manifest version."""

import os
import tomllib


package, separator, version = os.environ["RELEASE_TAG"].partition("/")
directories = {"py": "python", "rs": "rust"}
if not separator:
    version = package
    package = "both"
elif package not in directories or not version:
    raise SystemExit("Expected <version>, py/<version>, or rs/<version>")
selected = directories.values() if package == "both" else [directories[package]]
for directory in selected:
    with open(f"{directory}/Cargo.toml", "rb") as manifest:
        expected = tomllib.load(manifest)["package"]["version"]
    if version != expected:
        raise SystemExit(
            f"Tag version {version!r} does not match {directory}/Cargo.toml version {expected!r}"
        )
with open(os.environ["GITHUB_OUTPUT"], "a") as output:
    output.write(f"package={package}\n")
