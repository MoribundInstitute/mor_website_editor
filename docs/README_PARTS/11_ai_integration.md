## 🤖 AI & LLM Integration (Strictly Opt-In)

For developers and power users who want AI assistance without embedding a runtime inside the GUI, we maintain a standalone, headless MCP (Model Context Protocol) server in a separate repo.

By running the **MorWebsite MCP Engine**, you can connect your CLI agent or desktop IDE directly to MorWebsite core: open a website project, read pages, compile `mor-theme.css` through the editor's own pipeline, run the editor's diagnostics, and write validated theme presets.

Communication between the UI and the MCP server needs no IPC at all: the editor already hot-watches its `theme_presets/` folder (`theme_hot_reload` in the Dioxus app), so a preset the AI writes through the engine restyles the live preview immediately.

### Easiest: install from the editor

1. Open **Plugin Manager** (menu → Plugin Manager).
2. Open the **Marketplace** tab.
3. Click **Install MCP AI Bridge** (downloads the OS release asset and registers Claude Desktop + daemon registry).
4. Restart Claude Desktop / your MCP client so it picks up the registered server (`mor_website_engine`).

Release assets (exact names the installer expects):

| Platform | Asset |
|---|---|
| Linux | `mor-mcp-linux` |
| Windows | `mor-mcp-windows.exe` |
| macOS | `mor-mcp-macos` |

You can also install a local plugin folder from the CLI:

```bash
mwt plugin install /path/to/plugin_dir   # needs manifest.toml + entrypoint
# or from the MCP repo:
cd ../mor-website-editor-mcp && ./install.sh
```

### Manual agent registration

```bash
claude mcp add mor -- \
  /path/to/mor-mcp \
  --presets-dir /path/to/mor_website_editor/theme_presets
```

🔗 **Get the MCP Engine:** [mor-website-editor-mcp](https://github.com/MoribundMurdoch/mor-website-editor-mcp)

*Note: This is strictly opt-in. The core MorWebsite UI is offline and local by default. The engine is a separate process launched by your MCP client; the editor only helps download/register the binary and hot-reloads presets the agent writes.*
