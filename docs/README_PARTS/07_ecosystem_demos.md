## 🌐 Ecosystem & Lineage

MorWebsite Editor is a sibling/fork of the [MorBlogger Theme Editor](https://github.com/MoribundInstitute) — same socket-and-plug editor shell and preset system, retargeted from Blogger XML templates to regular local websites.

### Core Libraries
- [MOR UI Kit](https://github.com/MoribundInstitute/mor_rust_dioxus_ui_kit) — The standalone Dioxus UI toolkit powering the editor shell.

### Companion Plugins
Standalone tools that plug into the editor via its compiler crate (`mor_website_core`), checked out as sibling repos:
- [mor-website-editor-mcp](https://github.com/MoribundMurdoch/mor-website-editor-mcp) — the MCP engine for AI access (see *AI & LLM Integration* below).
- [mor-website-editor-ssh-publish](https://github.com/MoribundInstitute/mor-website-editor-ssh-publish) — `mor-publish`: export mor-theme.css and rsync the project to any SSH host (Hostinger defaults baked in).

### Legacy Lineage
The MorBlogger project maintains community compendiums (full themes, presets, widgets) built for Blogger. Its **theme presets** translate directly — MorWebsite uses the same `theme_presets/*.toml` format and `mor_` token namespace. Blogger-specific compendiums (widgets, XML structures) do not apply here.
