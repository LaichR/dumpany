{% if obj.display %}
   {% if is_own_page %}
{{ obj.short_name }}
{{ "=" * obj.short_name|length }}
.. py:module:: {{ obj.name }}

      {% if obj.docstring %}
.. autoapi-nested-parse::

   {{ obj.docstring|indent(3) }}

      {% endif %}

      {% block submodules %}
         {% set visible_subpackages = obj.subpackages|selectattr("display")
            |selectattr("type", "equalto", "package")
            |rejectattr("imported")|list %}
         {% set visible_submodules = obj.submodules|selectattr("display")|selectattr("type","equalto","module")|list %}
         {% set visible_submodules = (visible_subpackages + visible_submodules)|sort %}
         {% if visible_submodules %}
Submodules
----------
.. toctree::
   :maxdepth: 1

            {% for submodule in visible_submodules %}
   {{ submodule.include_path }}
            {% endfor %}


         {% endif %}
      {% endblock %}
    {% endif %}
{% endif %}
