# 🛠️ Getting Started

MorWebsite Editor is a Rust-powered desktop app for visually theming local websites and exporting a standalone `mor-theme.css`.

## Prerequisites

### 1. Install Rust

Install the Rust toolchain from:

- https://rustup.rs/

Verify installation:

```bash
rustc --version
cargo --version
```

### 2. Optional: PHP

If your site uses PHP, install it so the live preview can serve pages through `php -S`. Without PHP, the built-in static server is used instead.

---

## Option A: Launch the Visual Editor

```bash
cargo run -p mor_website_dioxus_ui
```

This opens the **native desktop window** (Dioxus desktop target — not a browser tab). For hot-reloading during development, install the Dioxus CLI (`cargo install dioxus-cli`) and run `dx serve` from `mor_website_dioxus_ui/`.

In the app: open your website folder, edit tokens in the dock panels, watch the live preview, and export `mor-theme.css` when you're happy.

---

## Option B: Use the Command-Line Tool (mwt)

Build the release executable:

```bash
cargo build --release -p mor_website_cli
```

The binary will be located at `target/release/mwt`.

```bash
# Initialize a MorWebsite workspace.toml in your website project
mwt init

# Or scaffold a full modular PHP starter (Site Contract + edit markers)
mwt init --template starter ./my-site

# Validate the theme: unresolved tokens, selector drift, unlinked stylesheets
mwt check --project .

# Compile tokens + modular CSS into mor-theme.css
mwt build --project .

# Package the themed site as a ZIP bundle
mwt bundle --project .

# Install a workspace plugin
mwt plugin install <path>
```

---

## Typical Development Workflow

```bash
# 1. Create or open a workspace
mwt init

# 2. Edit visually
cargo run -p mor_website_dioxus_ui

# 3. Validate
mwt check

# 4. Build mor-theme.css
mwt build

# 5. Package for distribution
mwt bundle
```

---

## Troubleshooting

### Build Failures

Update Rust:

```bash
rustup update
```

Clean and rebuild:

```bash
cargo clean
cargo build
```

### Theme Doesn't Apply

Run `mwt check` — the diagnostics will flag unresolved tokens and stylesheets your pages never link.
