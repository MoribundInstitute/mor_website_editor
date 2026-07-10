# Architectural Decision Records (ADR)

## Font Normalization Funnel

![Font normalization funnel](docs/diagrams/font_normalization_funnel.drawio.png)

**CONTEXT:** Users need flexibility with typography. They want to type known Google Font names, but they also want to drag-and-drop local `.ttf` or `.woff` files to test custom branding. The compiled theme is plain CSS (`mor-theme.css`) and needs standard font stacks / `@font-face` or hosted URL injection.

**DECISION:** The UI will allow multiple input methods (text typing, file drag-and-drop). However, the UI will *not* contain parsing logic. All inputs are immediately coerced into a raw string (the font name) and passed to `resolve_font_stack()` in `fonts.rs`. This single normalizer function coerces all input into standard CSS formatting.

**CONSEQUENCES:**
- UI remains flexible and frictionless (supports drag-and-drop).
- Codebase stays DRY (Don't Repeat Yourself).
- Only one parser to maintain in the core engine.
- Prevents bloat by not storing heavy binary font files in the theme state; we only store the extracted font name string.

## Website typography freedom (2026-07)

**CONTEXT:** MorWebsite is a normal website editor, not Blogger-only. Users need any family name, full CSS stacks, self-hosted `@font-face`, and a privacy-friendly webfont host.

**DECISION:**
- Free-form `font-family` strings always allowed (registry is a catalog, not a whitelist).
- `font_provider`: `bunny` (default) | `google` | `none` — remote faces inject as `@import` at the top of `mor-theme.css`.
- `custom_font_css` for user `@font-face` / extra imports.
- Expanded system + webfont catalog; weights 100–900.

**CONSEQUENCES:** Exported CSS is self-contained for remote fonts; self-hosting is first-class via None + custom CSS.

## Site Contract & modular markup (2026-07)

**CONTEXT:** The product themes hand-rolled HTML/PHP/JS sites (including optional web components). Over-abstracted page models fight GUI editing; sealed shadow DOM with hex colors fights live theming.

**DECISION:** Document and enforce a thin [Site Contract](docs/SITE_CONTRACT.md): DRY design tokens in `ThemeConfig` / CSS variables; WET structure via modules and PHP includes; stable `.mor-*` hooks; optional `data-mor-edit` / `data-field-path` markers for Editor Canvas. Ship `examples/mor_starter` and `mwt init --template starter`.

**CONSEQUENCES:**
- Some markup duplication is intentional and GUI-friendly.
- Web components must theme via CSS variables / `::part`, not closed hex styles.
- `ThemeConfig` remains the only source of truth for bound fields.

## Site-first product (not Blogger layout) (2026-07)

**CONTEXT:** Template Modules / Module Workbench / widget XML are Blogger-era. A real PHP site’s structure already lives in includes and CSS; a parallel slot graph confuses the golden path.

**DECISION:**
- **Designer mode (default):** open folder · tokens · Page · Insert · Inspector · Code · export CSS. No Template Modules accordion.
- **Advanced mode:** CSS/JS/diagnostics docks + **Starter kits (optional)** for scaffolding *new* pages only (`mor-starter.html`, copy HTML). Not a live layout engine.
- Structure source of truth = website folder on disk. Do not invest in Blogger-style drag-and-drop layout for this product.

**CONSEQUENCES:**
- `html_modules` / `template_pack` remain for starter export and token CSS folding.
- Module/Widget workbenches stay Advanced/legacy; Designer exits them when switching modes.
