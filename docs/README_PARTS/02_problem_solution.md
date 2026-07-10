# The Problem

Plenty of good websites are hand-rolled: a folder of HTML, PHP, CSS, and JS (maybe a web-component site), no bundler, no framework, no design-token pipeline. Restyling one means grepping for hex codes across a dozen stylesheets, editing, reloading, and hoping nothing drifted.

## ✨ The Solution

MorWebsite Editor gives those sites the visual theming workflow that framework projects take for granted:

1. **Open your site folder.** The editor serves it locally — `php -S` when PHP is installed, a built-in static server otherwise.
2. **Edit design tokens visually** — colors, typography, buttons, backgrounds, cursors, scrollbars, effects — in dockable panels, or hot-swap a preset from `theme_presets/*.toml`.
3. **See your real site live.** The generated stylesheet is injected into the preview as a `<style id="mor-true-css">` block and morphed in place as you edit — no destructive reloads, no scroll-jumping.
4. **Export.** The engine compiles everything into a single standalone `mor-theme.css` dropped into your project, or a full-site ZIP bundle.

Your site's own files stay the source of truth; the editor layers a token system on top and writes plain CSS back out.
