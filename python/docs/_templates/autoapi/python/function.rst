{% if obj.display %}
   {% if is_own_page %}
{{ obj.id }}
{{ "=" * obj.id | length }}

   {% endif %}
.. py:currentmodule:: {{obj.id|module_from_fully_qualified_name}}

.. py:function:: {{ obj.short_name }}({{ obj.args|shorten_qualnames }}){% if obj.return_annotation is not none %} -> {{ obj.return_annotation|shorten_qualnames }}{% endif %}
   {% for (args, return_annotation) in obj.overloads %}

                 {%+ if is_own_page %}{{ obj.id }}{% else %}{{ obj.short_name }}{% endif %}({{ args|shorten_qualnames }}){% if return_annotation is not none %} -> {{ return_annotation|shorten_qualnames }}{% endif %}
   {% endfor %}
   {% for property in obj.properties %}

   :canonical: {{ obj.id }}

   :{{ property }}:
   {% endfor %}

   {% if obj.docstring %}

   {{ obj.docstring|indent(3) }}
   {% endif %}
{% endif %}
