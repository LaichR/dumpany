{% import 'macros.rst' as macros %}
{% if obj.display %}
   {% if is_own_page %}

{{ obj.short_name|indent(0, True) }}
{{ "=" * obj.short_name|length }}

.. py:module:: {{ obj.id }}

      {% if obj.docstring %}
.. autoapi-nested-parse::

   {{ obj.docstring|indent(3) }}

      {% endif %}

      {% block content %}
         {% set visible_children = obj.children|selectattr("display")|list %}
         {% if visible_children %}
            {% set visible_attributes = visible_children|selectattr("type", "equalto", "data")|list %}
            {% if visible_attributes %}
               {% if "attribute" in own_page_types or "show-module-summary" in autoapi_options %}

                  {% if "attribute" in own_page_types %}
.. toctree::
   :hidden:

                     {% for attribute in visible_attributes %}
   {{ attribute.include_path }}
                     {% endfor %}

                  {% endif %}

{{ macros.auto_summary(visible_attributes, "Attributes") }}
               {% endif %}


            {% endif %}
            {% set visible_exceptions = visible_children|selectattr("type", "equalto", "exception")|list %}
            {% if visible_exceptions %}
               {% if "exception" in own_page_types or "show-module-summary" in autoapi_options %}

.. toctree::
   :hidden:

                     {% for exception in visible_exceptions %}
   {{ exception.include_path }}
                     {% endfor %}

{{ macros.auto_summary(visible_exceptions, "Exceptions") }}
               {% endif %}


            {% endif %}
            {% set visible_classes = visible_children
                                    |selectattr("type", "equalto", "class")
                                    |rejectattr("imported")|list %}
            {% if visible_classes %}
               {% if "class" in own_page_types or "show-module-summary" in autoapi_options %}


.. toctree::
   :hidden:

                     {% for klass in visible_classes %}
   {{ klass.include_path }}
                     {% endfor %}


{{ macros.auto_summary(visible_classes, "Classes")|indent(0, true) }}
               {% endif %}


            {% endif %}
            {% set visible_functions = visible_children|selectattr("type", "equalto", "function")|list %}
            {% if visible_functions %}
               {% if "function" in own_page_types or "show-module-summary" in autoapi_options %}

Functions
---------

                    {% for function in visible_functions %}
{{ function.render()|indent(0, true) }}
                    {% endfor %}

               {% endif %}

            {% endif %}
            {% set this_page_children = visible_children|rejectattr("type", "in", own_page_types)|list %}
            {% if this_page_children %}
            {% endif %}
         {% endif %}
      {% endblock %}
   {% else %}
.. py:module:: {{ obj.name }}

      {% if obj.docstring %}
   .. autoapi-nested-parse::

      {{ obj.docstring|indent(6) }}

      {% endif %}
      {% for obj_item in visible_children %}
   {{ obj_item.render()|indent(3) }}
      {% endfor %}
   {% endif %}
{% endif %}
