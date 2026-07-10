# MOR_PLAN: MorWebsite Editor

**Vision:** the definitive desktop tool for hand-rolled websites. A designer opens it
and never needs code; a developer opens it and gets a real editor with site-aware
intelligence. Every visual control and every line of code compile through the same
`ThemeConfig` → tokens/modules → `mor-theme.css` pipeline.

**The four laws:**
1. `ThemeConfig` is the brain. GUI panels and code editors are both just views of it.
2. Config-driven subsystems (buttons, scrollbars, cursors, palettes) are single sources
   of truth — presets and users never hand-write CSS the config can generate.
3. Ship no more than the page can use (module JS only when selected; diagnostics).
4. No `data-mor-edit` / `data-field-path` marker → no direct canvas edit. Preview is eyes;
   Editor Canvas is hands; config stays the brain. See [docs/SITE_CONTRACT.md](docs/SITE_CONTRACT.md).

---

## WHERE THE APP IS TODAY

* **Website plug:** open a local HTML/PHP/CSS/JS folder, serve with `php -S` or static,
  inject `mor-theme.css` live, export stylesheet or site ZIP.
* **Token-driven theming:** palette docks, presets (`theme_presets/*.toml`), modular CSS
  assembly (`build_master_css` + `render_css_sockets`).
* **Site-first structure:** open a real folder; PHP/HTML/CSS on disk is layout truth.
  Optional **Starter kits** (Advanced only) scaffold new pages — not a Blogger-style
  template pack for live sites.
* **Editor Canvas:** Edit | View | Browser; page text, Insert, Inspector; markers for
  shared site fields; tokens via Theme Palette.
* **Preview ribbon:** modes + Home/View/Selection tools (menu bar for app commands).
* **Designer mode (default):** Theme, Page, Insert, Inspector, Presets.
  View → Advanced Mode adds CSS/JS/diagnostics + starter kits.
* **CLI (`mwt`):** `init` / `init --template starter`, `check`, `build`, `bundle`, `plugin`.
* **Starter site:** `examples/mor_starter` + `scaffold_starter_site` (PHP includes + web component).
* **Diagnostics:** unresolved tokens, selector drift, unlinked stylesheets.
* **GTK import, plugins, optional MCP.**

---

## NEXT (product spine)

| Priority | Item |
|----------|------|
| Done | Identity cleanup (website, not Blogger) on user-facing surfaces |
| Done | Site Contract doc + starter scaffold |
| Done | Golden-path welcome + Designer mode |
| Done | Editor Canvas v1 markers on modules + starter |
| Next | Expand edit markers (menu labels/URLs, logo URL) |
| Next | First-run “Open starter” one-click from welcome when examples/ is present |
| Later | Preset/theme pack sharing; a11y contrast gate on export |

---

## Appendix — Editor Canvas binding

```html
<h1 data-mor-edit="site.site_title" data-field-path="site.site_title">My Site</h1>
```

Bridge events update `ThemeConfig`; renderer regenerates preview CSS/fields.
Never edit the iframe DOM as truth.
