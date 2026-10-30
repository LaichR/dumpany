{% macro _render_item_name(obj, sig=False) -%}
:py:obj:`{{ obj.name }} <{{ obj.id }}>`
     {%- if sig -%}
       \ (
       {%- for arg in obj.obj.args -%}
          {%- if arg[0] %}{{ arg[0]|replace('*', '\*') }}{% endif -%}{{  arg[1] -}}
          {%- if not loop.last  %}, {% endif -%}
       {%- endfor -%}
       ){%- endif -%}
{%- endmacro %}

{% macro _item(obj, sig=False, label='') %}
   * - {{ _render_item_name(obj, sig)|summary_link_style|indent(6) }}
     - {% if label %}:summarylabel:`{{ label }}` {% endif %}{% if obj|cls_docstring %}{{ obj.summary }}{% else %}\-{% endif +%}
{% endmacro %}

{% macro auto_summary(objs, title='') -%}
.. list-table:: {{ title }}
   :header-rows: 0
   :widths: auto
   :class: summarytable

  {% for obj in objs -%}
    {%- set sig = (obj.type in ['method', 'function'] and not 'property' in obj.properties) -%}

    {%- if 'property' in obj.properties -%}
      {%- set label = 'prop' -%}
    {%- elif 'classmethod' in obj.properties -%}
      {%- set label = 'class' -%}
    {%- elif 'abstractmethod' in obj.properties -%}
      {%- set label = 'abc' -%}
    {%- elif 'staticmethod' in obj.properties -%}
      {%- set label = 'static' -%}
    {%- else -%}
      {%- set label = '' -%}
    {%- endif -%}

    {{- _item(obj, sig=sig, label=label) -}}
  {%- endfor -%}

{% endmacro %}

{% macro render_elements( title, objects) -%}

{% if objects %}

{{title|indent(3, true)}}
   {{ "^" * title|length }}

    {% for obj in objects %}
{{ obj.render()| indent(3, true ) }}
    {% endfor %}
{% endif %}
{% endmacro %}
