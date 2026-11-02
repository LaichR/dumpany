"""Create a virtual environment and activate it for subsequent CI steps."""

import os
from pathlib import Path
import venv


def main():
    environment = Path(os.environ["GITHUB_WORKSPACE"]) / ".venv"
    venv.create(environment, with_pip=True)
    scripts = environment / ("Scripts" if os.name == "nt" else "bin")
    with open(os.environ["GITHUB_PATH"], "a", encoding="utf-8") as path_file:
        path_file.write(f"{scripts}\n")
    with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as env_file:
        env_file.write(f"VIRTUAL_ENV={environment}\n")


if __name__ == "__main__":
    main()
