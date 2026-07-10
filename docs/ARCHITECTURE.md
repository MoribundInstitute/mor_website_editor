# Architecture Overview

MorWebsite Editor separates theme compilation logic from the tools used to edit themes. The workspace is split into three crates with hard compile-time boundaries, and the Dioxus UI uses a reactive signal layer to drive rendering and export.

## BYOF Crate Boundaries

- **`mor_website_core`** — Headless engine: TOML preset parsing, CSS generation (`build_master_css`, `render_css_sockets`), website diagnostics (unresolved tokens, selector drift, unlinked stylesheets). No GUI or OS dependencies.
- **`mor_website_dioxus_ui`** — Visual workspace powered by Dioxus 0.7 and `mor_rust_dioxus_ui_kit`.
- **`mor_website_cli` (`mwt`)** — Terminal interface for init, check, build, bundle, and plugin.

Both frontends call into `mor_website_core` only.

## Socket & Plug UI Pattern

The Dioxus UI uses a generic **socket** (`MorLayoutChrome`, `DockZone`, dock chrome) and a domain-specific **plug** (`WebsiteWorkspace`). The sibling MorBlogger Theme Editor mounts a `BloggerWorkspace` plug on the same socket — forking for another domain means swapping the plug, not rebuilding the shell.

## Dioxus App: State, Docks & ThemeSignals

`App()` provides global context (`ThemeState`, `LayoutState`, `RenderState`, `VfsDictionary`). `ThemeSignals` is the reactive hub; dock panels read and write signals; `RenderState` derives memos that feed the preview and export pipeline.

## Theme Compile & Export Pipeline

`ThemeSignals.to_config()` produces a `ThemeConfig` memo. The core runs `build_master_css` over the modular CSS chunks and the active preset, `render_css_sockets` fills the remaining token sockets, and the result is written out as `mor-theme.css` (or packed into a full-site ZIP bundle).

## Live Preview

The editor serves your website folder locally — `php -S` when PHP is installed, a built-in static server otherwise — and injects the compiled theme into the page as a `<style id="mor-true-css">` block. As you edit, the block is morphed in place: no iframe reloads, no lost scroll position.

## Workspace UI Layout

`MorLayoutChrome` arranges the ActivityBar, left/right dock zones, the central website workspace, and floating undocked panels. `LayoutState` controls `DockPosition` and `CenterView`.

## Development Workflow

Visual editing (`cargo run -p mor_website_dioxus_ui`) and CLI workflows (`mwt`) converge on validated output from `mor_website_core`.

## Site Contract

Sites that work best with this editor follow a thin contract: CSS design tokens, stable `.mor-*` hooks, optional `data-mor-edit` markers for click-to-edit, and `mor-theme.css` as the export. See [SITE_CONTRACT.md](SITE_CONTRACT.md).

## Editable Sources

Diagram sources are stored as `.drawio` files in [`docs/diagrams/`](diagrams/). Several predate the fork from MorBlogger Theme Editor and depict the Blogger XML pipeline — treat those as legacy lineage.
