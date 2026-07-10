## 🤖 AI & LLM Integration (Strictly Opt-In)

For developers and power users who want AI assistance without embedding a runtime inside the GUI, we maintain a standalone, headless MCP (Model Context Protocol) server plus an in-editor **Robot Assist** policy.

By running the **MorWebsite MCP Engine**, you can connect your CLI agent or desktop IDE directly to MorWebsite core: open a website project, read/write pages, compile `mor-theme.css` through the editor's own pipeline, run diagnostics, scaffold sites, and write validated theme presets.

### Robot Assist (opt-in power)

Default is **off**. Enable in **Preferences → Robot Assist**:

| Tier | Agents may |
|------|------------|
| **Off** | No writes (reads still work if MCP is registered) |
| **Theme** | Presets + export `mor-theme.css` |
| **Site** (recommended) | Theme + write pages/CSS/JS, `workspace.toml`, inject theme links |
| **Full** | Site + scaffold starter, zip bundle, optional `delete_file` |

Policy file: `~/.config/mor_website/robot_assist.toml`  
Live session (for agents): `$XDG_RUNTIME_DIR/mor_website_session.json` or `/tmp/mor_website_session.json`

When Assist is on, the editor watches the open project so agent writes to `workspace.toml` / pages refresh the preview.

### Easiest setup

1. **Plugin Manager → Marketplace → Install MCP AI Bridge**
2. **Preferences → Robot Assist → Enable** (pick **Site** or **Full**)
3. Open your website folder in the editor
4. Restart Claude / Grok / your MCP client
5. Ask the agent: *“Call get_agent_handbook, then help me theme this site.”*

Release assets the installer expects:

| Platform | Asset |
|---|---|
| Linux | `mor-mcp-linux` |
| Windows | `mor-mcp-windows.exe` |
| macOS | `mor-mcp-macos` |

### Manual agent registration

```bash
claude mcp add mor -- \
  /path/to/mor-mcp \
  --presets-dir /path/to/mor_website_editor/theme_presets
```

Or from the MCP repo: `./install.sh` / `./install.sh --download`

### What agents can do (Full tier)

- `get_robot_policy` / `get_session` / `get_agent_handbook`
- `open_project`, `list_files`, `read_page`, `read_file`, `write_page`, `write_file`
- `write_site_config`, `apply_preset`, `write_preset`, `list_presets`
- `get_theme_css`, `export_theme_css`, `inject_theme_links`, `run_diagnostics`
- `scaffold_site`, `bundle_site`, `export_starter_page`, `list_modules`

🔗 **MCP Engine:** [mor-website-editor-mcp](https://github.com/MoribundMurdoch/mor-website-editor-mcp)

*Strictly opt-in. No embedded LLM. Agents are external processes; the editor only grants file/tool power and hot-reloads their writes.*
