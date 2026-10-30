Installation
============

The Python extension requires Python 3.10 or newer. It is not yet published on
PyPI; install a built wheel or build it from the repository.

Create a virtual environment
----------------------------

.. code-block:: console

   python -m venv .venv

Activate it using your shell's activation command:

.. tabs::

   .. tab:: Bash / Zsh

      .. code-block:: bash

         source .venv/bin/activate

   .. tab:: Fish

      .. code-block:: fish

         source .venv/bin/activate.fish

   .. tab:: PowerShell (Windows)

      .. code-block:: powershell

         .venv\Scripts\Activate.ps1

   .. tab:: Command Prompt (Windows)

      .. code-block:: bat

         .venv\Scripts\activate.bat

Install a wheel
---------------

If you have a wheel built for your platform, install it with pip (replace the
placeholder with the actual filename):

.. code-block:: console

   python -m pip install path/to/dumpany-wheel.whl

This does not require Rust. Wheels use the CPython stable ABI for Python 3.10+
and do not support free-threaded Python.

Build from source
-----------------

With Rust and its platform build tools installed, run these commands from the
repository root:

.. code-block:: console

   python -m pip install "maturin>=1.9,<2"
   cargo run --locked -p dumpany-python --bin generate_type_stubs
   maturin develop --manifest-path python/Cargo.toml

On Windows, the stub generator needs the Python installation directory on
``PATH`` so it can load the Python shared library.

Check the installation
----------------------

.. code-block:: console

   python -c "import dumpany; print(dumpany.__version__)"

Continue with :doc:`getting-started`.
