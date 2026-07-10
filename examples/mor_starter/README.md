# Mor starter site

Modular PHP + optional web component, wired for MorWebsite Editor.

## Open in the editor

1. Launch MorWebsite Editor (`cargo run -p mor_website_dioxus_ui`).
2. **File → Open Website Folder…** and pick this directory.
3. Left dock → **Presets** (or Theme Palette) → load a look.
4. Preview mode → **Edit** → double-click the site title.
5. **File → Save Theme to Site** (writes `workspace.toml` + `mor-theme.css`).

## Layout

- `includes/header.php` / `footer.php` — shared chrome (WET pages, DRY includes)
- `components/mor-card.js` — themable custom element
- `css/site.css` — local rules using CSS variables
- `workspace.toml` — editor ThemeConfig
- `mor-theme.css` — generated on export / `mwt build`

See `docs/SITE_CONTRACT.md` in the MorWebsite Editor repo.
