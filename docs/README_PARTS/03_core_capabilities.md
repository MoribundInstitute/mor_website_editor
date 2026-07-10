## 🚀 Core Capabilities

### 🎨 Token-Driven Theming
* **Theme palette docks:** Visual panels for colors, typography, buttons, backgrounds, cursors, scrollbars, and effects — every control writes to the same `ThemeConfig`.
* **Presets:** Load, tweak, and save complete looks as `theme_presets/*.toml`. Swap the whole skin in one click.
* **Clean CSS assembly:** `build_master_css` + `render_css_sockets` stitch your tokens and modular CSS into a single `mor-theme.css` (see [CSS Pipeline](docs/CSS_PIPELINE.md)).

### 🖥️ Your Real Site, Live
* **Local preview server:** Your project folder is served as-is — `php -S` when PHP is available, a built-in static server otherwise. PHP sites render as PHP, not as source dumps.
* **Live injection:** The generated stylesheet rides in a `<style id="mor-true-css">` block and is morphed in place as you edit — no iframe reloads.
* **CSS/JS editors on real files:** CodeMirror editors and the CSS/JS builders operate on your site's actual stylesheets and scripts.
* **Static page stencils:** Generate standalone HTML page scaffolds that pick up the active theme.

### 🩺 Website-Flavored Diagnostics
* **Unresolved tokens:** Theme variables referenced but never defined.
* **Selector drift:** Theme rules targeting selectors your site no longer contains.
* **Unlinked stylesheets:** CSS files in the project that no page actually loads.

### 🎁 Linux Desktop Integration (GTK Import)
* **Match your desktop:** Import color schemes and visual styles from GTK3/GTK4 themes like Adwaita, Nord, or WhiteSur.
* **Built-in graphics:** Bundled SVGs are converted to data URIs and embedded straight into the theme — no extra HTTP requests.

### 🧰 Extensibility
* **Plugin manager:** Install workspace plugins in-app or via `mwt plugin install`.
* **Opt-in MCP/AI bridge:** Connect an external MCP engine when you want it; fully offline otherwise.

### 🪟 The Visual Workspace
* **Lightweight interface:** Built on Dioxus 0.7 — a fast, native desktop shell.
* **Dockable everything:** Pinnable activity bar, floating panels, flexible dock zones.
* **Adaptive window chrome:** Borders adapt to Windows, macOS, and keyboard-only Linux setups.
