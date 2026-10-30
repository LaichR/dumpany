{% if obj.display %}

.. py:property:: {{ obj.short_name }}
   {% if obj.annotation %}
   :type: {{ obj.annotation | shorten_type_expr }}
   {% endif %}
   {% for property in obj.properties %}
   :{{ property| filter_properties }}:
   {% endfor %}

   {% if obj.docstring %}

   {{ obj.docstring|indent(3) }}
   {% endif %}
{% endif %}
