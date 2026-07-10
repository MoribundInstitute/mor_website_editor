# MorWebsite Site Contract

A short contract for sites that work well with MorWebsite Editor. Follow it and the GUI can restyle, diagnose, and (where marked) click-to-edit your pages. Break it and you still have a website — you just lose editor superpowers.

## One-sentence product

Open any hand-rolled site. Change how it looks without grepping hex codes. Export one CSS file (`mor-theme.css`).

## Layers

| Layer | Who owns it | What it is |
|-------|-------------|------------|
| **Tokens** | Editor / `ThemeConfig` | CSS custom properties: `--bg-base`, `--accent`, `--font-heading`, … |
| **Hooks** | Your markup | Stable classes: `.mor-topbar`, `.mor-card`, `.mor-pill`, … |
| **Slots** | Template pack | header / sidebar / footer module choice |
| **Edit markers** | Optional in your HTML | `data-mor-edit` / `data-field-path` → config paths |
| **Output** | Export | `mor-theme.css` (+ optional `mor-theme.js`) |

## DRY vs WET (modularity for GUIs)

| Share (DRY) | Duplicate freely (WET) |
|-------------|------------------------|
| Colors, fonts, radii, spacing | Page-level HTML / PHP shells |
| Module CSS/JS behind a slot | Slightly different footer as another module |
| `ThemeConfig` / preset TOML | Per-page content text |
| Class naming contract | Copy-paste of a card block on three pages |

**DRY tokens. WET structure.** Abstraction only a senior engineer understands hurts GUI editing. Named modules the GUI can swap help.

## Tokens

Link the compiled stylesheet on every page:

```html
<link rel="stylesheet" href="/mor-theme.css" />
```

Prefer variables over hex in your own CSS:

```css
.hero {
  background: var(--bg-panel);
  color: var(--fg-base);
  border-color: var(--border-color);
}
.hero h1 { font-family: var(--font-heading); color: var(--accent); }
```

## Hooks (`.mor-*`)

Structural modules ship classes under the `mor-` prefix. Use them (or keep them when you customize modules) so:

1. Theme CSS can target layout chrome reliably.
2. Diagnostics can detect **selector drift** (theme rules with no matching class in the site).
3. Presets restyle every module without per-page rewrites.

Examples: `.mor-topbar`, `.mor-brand`, `.mor-pill`, `.mor-sidebar-nav`, `.mor-footer-grid`, `.mor-card`.

## Edit markers (Editor Canvas)

The preview never treats the iframe DOM as source of truth. Bound fields carry a path back into `ThemeConfig`:

```html
<span data-mor-edit="site.site_title" data-field-path="site.site_title">My Site</span>
<p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle">Tagline</p>
<p data-mor-edit="footer.footer_text" data-field-path="footer.footer_text">© You</p>
```

| Attribute | Role |
|-----------|------|
| `data-mor-edit` | Preferred public name (Site Contract) |
| `data-field-path` | Alias used by the canvas bridge (same paths) |

**Supported paths (v1):**

- `site.site_title`
- `site.site_subtitle`
- `footer.footer_text`
- `typography.body_font_stack` / `heading_font_stack` / `mono_font_stack`
- `icons.*` (shift-click / SVG drop)
- Theme token surfaces via `data-edit-target="colors.accent"` etc. (Inspect mode focuses the palette)

**Rule:** no marker → no direct edit. Inspect may classify unbound DOM (typography/colors panels) but will not invent config bindings.

In the editor: switch preview mode to **Edit**, double-click a marked field, blur/Enter → config updates → CSS/preview refresh.

## Web components

Fine for modular interactive bits **if** they theme from the outside:

```html
<mor-card class="mor-card">
  <span slot="title">Hello</span>
</mor-card>
```

```css
mor-card {
  --card-bg: var(--bg-panel);
  --card-fg: var(--fg-base);
  --card-accent: var(--accent);
}
```

Avoid baking hex colors into closed shadow styles. Expose `::part()` or CSS variables. Filename convention for the editor’s component linker: `components/my-card.php`, `css/my-card.css`, `js/my-card.js` matching the tag name.

## PHP includes

Cheap modularity, works with `php -S` preview:

```
site/
  index.php
  about.php
  includes/header.php
  includes/footer.php
  components/mor-card.js
  mor-theme.css          ← editor export
  workspace.toml         ← optional editor state
```

Pages may duplicate a thin shell; put shared chrome in includes or Mor modules.

## Diagnostics the contract enables

| Check | Meaning |
|-------|---------|
| Unresolved tokens | Variables referenced but never defined |
| Selector drift | Theme rules for classes the site no longer has |
| Unlinked stylesheets | CSS files no page loads |

Run `mwt check --project .` before deploy.

## Starter project

Open [`examples/mor_starter`](../examples/mor_starter) in MorWebsite Editor (`File → Open Website Folder…`), or scaffold with:

```bash
mwt init --template starter ./my-site
```

## Editor chrome

| Surface | Role |
|---------|------|
| **Menu bar** | Full app commands (File, Edit, View, Docks, …) |
| **Preview ribbon** | Browse / Inspect / Edit + contextual canvas tools (colors, device frame, selection) |
| **Activity bar** | Dock icons — pin/unpin docks via right-click (taskbar model) |

## What not to do

- Freeform Wix-style layout as source of truth (this app edits a **theme-compiler** model).
- Sealed shadow DOM full of hex colors.
- Expecting the editor to reverse-engineer arbitrary third-party CMSs without hooks.
- Shipping without linking `mor-theme.css` on real pages (export writes the file; you still link it — or use inject / the starter).

---

See also: [Architecture](ARCHITECTURE.md), [CSS Pipeline](CSS_PIPELINE.md), [Theme Creation](THEME_CREATION.md).
