dumpany
=======

Generate structured, width-aware text using composable dump operations.

When generating reports, documentation, or source code, formatting often gets
mixed into the logic that visits your data. dumpany lets you describe text,
lists, indentation, and aligned blocks as operations, then takes care of laying
them out. Generators can produce these operations as they traverse your objects.

For a single line of text, ``print()`` is enough. dumpany is useful when the
output has structure: wrapped list items, nested blocks, or a header whose
contents become known only after visiting the body.

.. toctree::
   :maxdepth: 2
   :caption: Contents

   installation
   getting-started
   examples
   api

Indices
-------

* :ref:`genindex`
* :ref:`search`
