{% import 'macros.rst' as macros %}
{% if obj.display %}
   {% if is_own_page %}
.. py:currentmodule:: {{  obj.id | module_from_fully_qualified_name }}

{{ obj.short_name }}
{{ "=" * obj.short_name | length }}

   {% endif %}
   {% set visible_children = obj.children
            |selectattr("display")
            |rejectattr("imported")
            |rejectattr("inherited")
            |list %}
   {% set own_page_children = visible_children|selectattr("display")|selectattr("type", "in", own_page_types)|list %}
   {% if is_own_page and own_page_children %}
.. toctree::
   :hidden:

      {% for child in own_page_children %}
   {{ child.include_path }}
      {% endfor %}

   {% endif %}
   {% set new_methods = obj.children|selectattr("short_name", "equalto", "__new__")|list %}
   {% set constructor_args = obj.args or (new_methods[0].args if new_methods else '') %}
.. py:{{ obj.type }}:: {{ obj.short_name }}{% if constructor_args %}({{ constructor_args|shorten_qualnames }}){% endif %}
{%if is_own_page %}

   :canonical: {{ obj.id }}
{% endif %}

   {% for (args, return_annotation) in obj.overloads %}
      {{ " " * (obj.type | length) }}   {{ obj.short_name }}{% if args %}({{ args|shorten_qualnames }}){% endif %}

   {% endfor %}
   {% if obj.bases %}
{{ obj.bases | base_references | indent(3, true) }}
   {% endif %}
   {% if obj.docstring %}

{{ obj|cls_docstring|indent(3, true) }}
    {% endif %}
    {% if is_own_page %}
        {% set visible_methods = visible_children|selectattr("type", "equalto", "method")|list %}
        {{ macros.render_elements("Methods", visible_methods) }}

        {% set visible_properties = visible_children|selectattr("type", "equalto", "property")|list %}
        {{ macros.render_elements("Properties", visible_properties) }}

        {% set visible_attributes = visible_children|selectattr("type", "equalto", "attribute")|list %}
        {{ macros.render_elements("Attributes", visible_attributes) }}

        {% set visible_exceptions = own_page_children|selectattr("type", "equalto", "exception")|list %}
        {{ macros.render_elements("Exceptions", visible_exceptions) }}
      {% endif %}
{% endif %}
