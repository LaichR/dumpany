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

Choosing join breaks
--------------------

Use ``Join(..., breaks=...)`` to choose safe break positions between complete
items, including inside a ``NoBreakRegion``:

.. code-block:: python

   from dumpany import BreakPolicy, Join, NoBreakRegion

   dumper.dump(NoBreakRegion([
       "call(",
       Join(", ", arguments, breaks=BreakPolicy.AsNeeded),
       ")",
   ]))

``Inherit`` is the default: the join inserts no breaks of its own, and items and
delimiters follow the surrounding wrapping rules. ``Always`` breaks after each
delimiter; ``AsNeeded`` does so when the next item's first line will not fit.
The first item starts at the current position. Explicit policies keep items and
delimiters intact except for their own explicit newlines. A delimiter stays
attached to the preceding item even if that exceeds the width.

Prologues and epilogues count toward an item's width, even when supplied later.
Iterators and callbacks run once when the join is dumped. Contexts and regions
must balance within each item or delimiter; target switching and indentation
changes are not allowed there. Errors fail the dumper without undoing output
already written.
