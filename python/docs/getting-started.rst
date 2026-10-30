Getting started
===============

A release checklist
-------------------

Suppose you are generating a checklist. Each item needs a prefix, and long
items should wrap without repeating that prefix. This is what a
:py:class:`~dumpany.Context` provides:

.. code-block:: python

   import io

   from dumpany import Context, Dumper

   tasks = [
       "Build the Rust and Python packages",
       "Run the tests",
   ]
   stream = io.StringIO()
   dumper = Dumper(stream, line_width=24)
   dumper.dump("Release checklist\n")
   dumper.dump(Context(task, prefix="- ") for task in tasks)
   dumper.dump_all()
   print(stream.getvalue(), end="")

Output:

.. code-block:: text

   Release checklist
   - Build the Rust and
     Python packages
   - Run the tests

Each context ends its own block. Continuation lines align after the ``"- "`` prefix;
you do not have to calculate the padding or split the text into lines yourself.

How the pieces fit together
---------------------------

* :py:class:`~dumpany.Dumper` writes to a text stream such as ``io.StringIO``,
  a text file, or ``sys.stdout``.
* :py:meth:`~dumpany.Dumper.dump` accepts strings, operation objects, and nested
  iterables. Other objects are converted with ``str()``. Iterables have no
  implicit separators; mappings yield their keys rather than being serialized.
* Operations describe formatting rather than producing strings themselves.
  For example, ``Context`` groups aligned content, ``NewLine`` controls line
  breaks and indentation, and ``Join`` inserts separators.
* :py:meth:`~dumpany.Dumper.dump_all` finishes the output and flushes the stream
  without closing it. Call it explicitly; after finalization, no further text
  may be added. Output can be buffered before this call.

See :doc:`examples` for separated content and a complete YAML example, or
:doc:`api` for signatures and argument descriptions.
