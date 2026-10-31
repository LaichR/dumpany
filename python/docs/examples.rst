Examples
========

Separating generated values
---------------------------

``Join`` accepts more than strings: its items and delimiter can themselves be
dump operations. Here it generates function parameters inside a context whose
``"("`` prefix aligns wrapped lines after the opening parenthesis.
Closing that context increases the indentation for the function body:

.. code-block:: python

   import io

   from dumpany import Context, Dumper, Join, NewLine

   parameters = [
       ("name", "str"),
       ("age", "int"),
       ("email_address", "str"),
       ("street", "str"),
       ("city", "str"),
       ("zip_code", "str"),
       ("country", "str"),
   ]
   stream = io.StringIO()
   dumper = Dumper(stream, line_width=78)
   dumper.dump([
       "def register",
       Context([
           Join(", ", ([name, ": ", annotation] for name, annotation in parameters)),
           "):",
       ], prefix="(", increment=1),
       "...",
       NewLine(decrement=1),
   ])
   dumper.dump_all()
   print(stream.getvalue(), end="")

Output:

.. code-block:: python

   def register(name: str, age: int, email_address: str, street: str, city: str,
                zip_code: str, country: str):
       ...

The parameter context handles continuation alignment. Closing it ends the
signature's final line and then increases normal indentation by one level, so
the body starts four spaces in without an extra blank line. The final newline
decreases indentation again.

Putting it all together
-----------------------

This example generates a YAML mapping for a school class: its name, the pupil
count, and an array of pupils with names and surnames.

``class_yaml()`` receives an iterable, not necessarily a list. It does not know
the pupil count in advance. It counts pupils while producing the list, then
uses ``Prologue`` to place the completed count above that list.

.. literalinclude:: examples/pupils.py
   :language: python

Output (apart from trailing blank lines):

.. code-block:: yaml

   name: "Class 4B"
   pupil_count: 2
   pupils:
       - name: "Alice"
         surname: "Smith"
       - name: "Ben"
         surname: "Jones"

* ``NewLine(increment=1)`` indents the list by four spaces. The matching
  decrement restores the previous indentation.
* ``Context(prefix="- ")`` introduces each pupil and aligns ``surname`` with
  ``name`` without repeating the dash.
* The named outermost context stays open while the generator runs, so the
  prologue can still add the header after the last pupil has been visited.
* ``json.dumps()`` quotes names using YAML-compatible double-quoted strings.
  An empty iterable produces ``pupil_count: 0`` and ``pupils: []``.

To place content after a named block instead, use
:py:class:`~dumpany.Epilogue`. For example, a report generator could discover
footnotes while visiting its sections and append them after the report body.
