# The CSS Assembly Pipeline

MorWebsite Editor compiles dozens of modular CSS sources into a single standalone `mor-theme.css`. All community presets should use the `mor_` variable namespace.

## Inputs

| Source | Description |
|--------|-------------|
| Modular CSS chunks | Shipped subsystem stylesheets (buttons, scrollbars, cursors, backgrounds, effects) |
| `preset_css` | Active preset stylesheet bound to `ThemeSignals` |
| `VfsDictionary` | User custom CSS loaded from the workspace |

## Processing (`css_builder.rs`)

1. **Strip** — Remove stray markup wrappers from pasted CSS.
2. **Sanitize** — Slice and validate individual modules without nesting errors.
3. **Stitch** — `build_master_css` merges everything into `mor_`-namespaced rules; `render_css_sockets` resolves the remaining token sockets against the active `ThemeConfig`.

## Output

The compiled stylesheet is exported as **`mor-theme.css`** into your website project (link it from your pages), or bundled with the whole site as a ZIP. In the live preview, the same CSS is injected as a `<style id="mor-true-css">` block and morphed live as you edit.

## Related

- [Architecture Overview](ARCHITECTURE.md) — Full compile and export flow
- [Theme Creation Guide](THEME_CREATION.md) — Naming conventions for preset authors
