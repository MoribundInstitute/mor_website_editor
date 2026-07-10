## 🤖 AI & LLM Integration (Strictly Opt-In)

For developers and power users who want AI assistance without embedding a runtime inside the GUI, we maintain a standalone, headless MCP (Model Context Protocol) server in a separate repo.

By running the **MorWebsite MCP Engine**, you can connect your CLI agent or desktop IDE directly to MorWebsite core: open a website project, read pages, compile `mor-theme.css` through the editor's own pipeline, run the editor's diagnostics, and write validated theme presets.

Communication between the UI and the MCP server needs no IPC at all: the editor already hot-watches its `theme_presets/` folder (`theme_hot_reload` in the Dioxus app), so a preset the AI writes through the engine restyles the live preview immediately. Register the engine with your MCP client (e.g. `claude mcp add mor -- …/mor-mcp --presets-dir …/theme_presets`) — setup details in its README.

🔗 **Get the MCP Engine:** [mor-website-editor-mcp](https://github.com/MoribundInstitute/mor-website-editor-mcp)

*Note: This is strictly opt-in. The core MorWebsite UI is offline and local by default — the engine is a separate process you launch through your MCP client, never from inside the editor.*
