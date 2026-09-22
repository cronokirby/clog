# clog

I felt like writing a static site builder in C.

## Collections

Use `collections` in a page's YAML frontmatter to name the magazines, anthologies,
or other collections it belongs to. It accepts a single string or a list:

```yaml
collections: "[[Paris Review 26]]"
```

```yaml
collections:
  - "[[Paris Review 26]]"
  - "[[Collected Stories|Collected Stories, Volume 1]]"
```

Quote wikilinks so YAML treats them as strings. Each wikilink resolves to an
existing page, including through its aliases. Missing targets display as plain
text, and plain strings are displayed without creating links. Collections do not
create pages automatically. Existing collection pages list referring pages in
their backlinks, just like links in the body.

Page templates receive `collections` as a list of rendered HTML strings, like
`authors`. Missing, null, or empty collections produce an empty list. To display
them, add this to `templates/index.html` (also included in the example template):

```html
{% if collections %}
<div class="collections">
  In {% for collection in collections %}{{ collection }}{% if not loop.last %}, {% endif %}{% endfor %}
</div>
{% endif %}
```
