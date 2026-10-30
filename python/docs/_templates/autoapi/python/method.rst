{% if obj.display %}
   {% if is_own_page %}
{{ obj.id }}
{{ "=" * obj.id | length }}

   {% endif %}
.. py:method:: {{ obj.short_name }}({{ obj.args|shorten_qualnames }}){% if obj.return_annotation is not none %} -> {{ obj.return_annotation|shorten_qualnames }}{% endif %}
   {% for (args, return_annotation) in obj.overloads %}

               {%+ if is_own_page %}{{ obj.id }}{% else %}{{ obj.short_name }}{% endif %}({{ args|shorten_qualnames }}){% if return_annotation is not none %} -> {{ return_annotation|shorten_qualnames }}{% endif %}
   {% endfor %}

   :canonical: {{obj.id}}
   {% for property in obj.properties %}
   :{{ property| filter_properties }}:
   {% endfor %}

   {% if obj.docstring %}

   {{ obj.docstring|indent(3) }}
   {% endif %}
{% endif %}
