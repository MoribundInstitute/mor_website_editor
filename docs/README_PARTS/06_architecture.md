## Architecture: Bring Your Own Frontend (BYOF)

MorWebsite Editor splits the software into a reusable **Socket** (window chrome, docks, activity bar) and a swappable **Plug** (website-specific preview, export, and project views). The theme engine stays in `mor_website_core`; both the Dioxus UI and `mwt` CLI call into it.

### The Three Main Pieces

* **1. The Engine (`mor_website_core`):** Headless logic — reads settings and presets, builds the master CSS, runs diagnostics. No buttons or windows.
* **2. The Visual Workspace (`mor_website_dioxus_ui`):** The desktop UI you click on. Socket-and-plug layout:
  * **Socket (`MorLayoutChrome`):** Main window, dock zones, activity bar, floating panels.
  * **Plug (`WebsiteWorkspace`):** Live site preview, theme export, and project views. This is the same socket the sibling MorBlogger Theme Editor uses — only the plug differs.
* **3. The Command Line (`mor_website_cli` / `mwt`):** Terminal workflow for init, check, build, bundle, and plugin.

### Socket & Plug in Plain English

| Piece | What it is | Key types |
| --- | --- | --- |
| **Socket** | Generic editor shell — tabs, docks, popups | `MorLayoutChrome`, `DockZone`, `ActivityBar` |
| **Plug** | Domain brains — what you are actually editing | `WebsiteWorkspace` here; `BloggerWorkspace` in the sibling MorBlogger editor |
| **Promise** | Swap the plug, keep the socket | Fixes in website logic don't require rewriting dock chrome |

Deep dive: [Architecture Overview](docs/ARCHITECTURE.md)
