use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use mor_website_core::presets::theme_config_from_path;
use mor_website_core::website::{
    check_website, export_theme_bundle, generate_theme_css, scan_project, theme_js_snippet,
    theme_link_snippet,
    zip_site,
};

#[derive(Parser)]
#[command(
    name = "mwt",
    about = "Headless theme compiler for Moribund website projects",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a workspace.toml, or a full modular starter site (`--template starter`).
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// `minimal` = workspace.toml only; `starter` = modular PHP site (Site Contract).
        #[arg(short, long, default_value = "minimal")]
        template: String,
        #[arg(short, long)]
        force: bool,
    },
    /// Scan a website folder and run theme/markup integrity checks.
    Check {
        #[arg(short, long, default_value = "workspace.toml")]
        input: PathBuf,
        /// Website project directory (folder of HTML/PHP/CSS/JS).
        #[arg(long)]
        project: PathBuf,
    },
    /// Generate the theme CSS from a workspace TOML.
    Build {
        #[arg(short, long, default_value = "workspace.toml")]
        input: PathBuf,
        /// Output path, or "-" to write CSS to stdout.
        #[arg(short, long, default_value = "mor-theme.css")]
        output: PathBuf,
        /// Website project directory; writes the CSS into it instead of --output.
        #[arg(long, conflicts_with = "output")]
        project: Option<PathBuf>,
    },
    /// Zip a website project (with freshly generated theme CSS) for deployment.
    Bundle {
        #[arg(short, long, default_value = "workspace.toml")]
        input: PathBuf,
        /// Website project directory (folder of HTML/PHP/CSS/JS).
        #[arg(long)]
        project: PathBuf,
        #[arg(short, long, default_value = "site_bundle.zip")]
        output: PathBuf,
    },
    /// Install a local MCP plugin directory (manifest.toml + entrypoint) into editor prefs and daemon registry.
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Register a local MCP plugin without recompiling the editor workspace.
    Install {
        /// Path to plugin folder containing manifest.toml and entrypoint script/binary.
        path: PathBuf,
    },
    /// List MCP binaries and config-bridge plugin registrations.
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let start_time = Instant::now();

    match &cli.command {
        Commands::Init {
            path,
            template,
            force,
        } => {
            let target_file = path.join("workspace.toml");
            let starter = mor_website_core::config::defaults::default_theme_config();

            match template.as_str() {
                "starter" => {
                    if path.join("index.php").exists() && !force {
                        anyhow::bail!(
                            "Starter site already looks present at {}. Use --force to overwrite.",
                            path.display().to_string().yellow()
                        );
                    }
                    println!(
                        "{} Scaffolding modular starter site in {:?}...",
                        "➔".cyan(),
                        path
                    );
                    fs::create_dir_all(path).ok();
                    let written = mor_website_core::website::scaffold_starter_site(path, &starter)
                        .with_context(|| {
                            format!("Failed to scaffold starter site at '{}'", path.display())
                        })?;
                    println!(
                        "{} Starter site ready ({} files). Open the folder in MorWebsite Editor.",
                        "✔".green().bold(),
                        written.len()
                    );
                    println!(
                        "{} See docs/SITE_CONTRACT.md for hooks, tokens, and edit markers.",
                        "➔".cyan()
                    );
                }
                "minimal" | _ => {
                    if template != "minimal" && template != "starter" {
                        eprintln!(
                            "{} Unknown template '{}'; using 'minimal'. Known: minimal, starter.",
                            "⚠".yellow(),
                            template
                        );
                    }
                    if target_file.exists() && !force {
                        anyhow::bail!(
                            "Workspace already exists at {}. Use --force to overwrite.",
                            target_file.display().to_string().yellow()
                        );
                    }

                    println!(
                        "{} Initializing new 'minimal' workspace in {:?}...",
                        "➔".cyan(),
                        path
                    );

                    let toml_bytes = toml::to_string_pretty(&starter)
                        .context("Failed to serialize starter workspace config")?;
                    if let Some(parent) = target_file.parent() {
                        fs::create_dir_all(parent).ok();
                    }
                    fs::write(&target_file, toml_bytes.as_bytes())
                        .with_context(|| format!("Failed to write {}", target_file.display()))?;

                    println!(
                        "{} Workspace ready at {}. Tip: `mwt init --template starter` for a full modular PHP site.",
                        "✔".green().bold(),
                        target_file.display()
                    );
                }
            }
        }
        Commands::Check { input, project } => {
            println!("{} Reading configuration...", "➔".cyan());
            let config = load_config(input)?;

            println!(
                "{} Scanning website project at {}...",
                "➔".cyan(),
                project.display().to_string().yellow()
            );
            let site = scan_project(project).with_context(|| {
                format!("Failed to scan website project at '{}'", project.display())
            })?;

            println!("{} Running website integrity checks...", "➔".cyan());
            let diag = check_website(&site, &config);

            for error in &diag.errors {
                println!("  {} {}", "✘".red().bold(), error);
            }
            for warning in &diag.warnings {
                println!("  {}", warning.format_line());
            }

            if !diag.is_valid {
                anyhow::bail!(
                    "Website integrity failure! Fix the reported errors before deploying."
                );
            }
            println!(
                "{} Website project is valid ({} warning(s)) in {:?}",
                "✔".green().bold(),
                diag.warnings.len(),
                start_time.elapsed()
            );
        }
        Commands::Build {
            input,
            output,
            project,
        } => {
            println!("{} Reading configuration...", "➔".cyan());
            let config = load_config(input)?;

            if let Some(project) = project {
                let site = scan_project(project).with_context(|| {
                    format!("Failed to scan website project at '{}'", project.display())
                })?;
                let written = export_theme_bundle(&site, &config).with_context(|| {
                    format!("Failed to write theme files into '{}'", project.display())
                })?;
                println!(
                    "{} Theme files written to {} ({}) in {:?}",
                    "✔".green().bold(),
                    project.display().to_string().yellow(),
                    written.join(", "),
                    start_time.elapsed()
                );
                let mut snippets = theme_link_snippet();
                if written.iter().any(|f| f.ends_with(".js")) {
                    snippets.push_str("\n  ");
                    snippets.push_str(&theme_js_snippet());
                }
                println!(
                    "{} Link them from your pages with:\n  {}",
                    "➔".cyan(),
                    snippets
                );
            } else {
                println!("{} Generating theme CSS...", "➔".cyan());
                let css = generate_theme_css(&config);

                if output.as_os_str() == "-" {
                    use std::io::Write;
                    std::io::stdout()
                        .write_all(css.as_bytes())
                        .context("Failed to write CSS to stdout")?;
                } else {
                    if let Some(parent) = output.parent() {
                        if !parent.as_os_str().is_empty() {
                            fs::create_dir_all(parent).with_context(|| {
                                format!("Failed to create output directory '{}'", parent.display())
                            })?;
                        }
                    }
                    fs::write(output, css.as_bytes()).with_context(|| {
                        format!("Failed to write CSS to '{}'", output.display())
                    })?;
                    println!(
                        "{} Theme CSS compiled to {} in {:?}",
                        "✔".green().bold(),
                        output.display().to_string().yellow(),
                        start_time.elapsed()
                    );
                }
            }
        }
        Commands::Bundle {
            input,
            project,
            output,
        } => {
            println!("{} Reading configuration...", "➔".cyan());
            let config = load_config(input)?;

            println!(
                "{} Scanning website project at {}...",
                "➔".cyan(),
                project.display().to_string().yellow()
            );
            let site = scan_project(project).with_context(|| {
                format!("Failed to scan website project at '{}'", project.display())
            })?;

            println!("{} Archiving site with generated theme CSS...", "➔".cyan());
            zip_site(&site, &config, output).with_context(|| {
                format!("Failed to write site bundle to '{}'", output.display())
            })?;

            println!(
                "{} Site bundle written to {} in {:?}",
                "✔".green().bold(),
                output.display().to_string().yellow(),
                start_time.elapsed()
            );
        }
        Commands::Plugin { command } => match command {
            PluginCommands::Install { path } => {
                println!(
                    "{} Installing local MCP plugin from {}...",
                    "➔".cyan(),
                    path.display().to_string().yellow()
                );

                let report = mor_website_core::utils::mcp_install::install_local_mcp_plugin(&path)
                    .map_err(|e| anyhow::anyhow!(e))?;

                println!(
                    "{} Plugin '{}' registered in config bridge",
                    "✔".green().bold(),
                    report.plugin_id
                );
                println!(
                    "  binary: {}",
                    report.binary_path.display().to_string().yellow()
                );
                println!(
                    "  daemon registry: {}",
                    report.daemon_registry.display().to_string().yellow()
                );
                println!(
                    "  editor prefs: {}",
                    report.editor_prefs.display().to_string().yellow()
                );
                if let Some(claude_path) = report.claude_config {
                    println!(
                        "  claude MCP config: {}",
                        claude_path.display().to_string().yellow()
                    );
                }

                let registry =
                    mor_website_core::utils::mcp_install::read_daemon_registry().map_err(|e| {
                        anyhow::anyhow!(e)
                    })?;
                if let Some(server) = registry["servers"].get(&report.plugin_id) {
                    if let Some(prompt) = server.get("system_prompt").and_then(|v| v.as_str()) {
                        println!("  system_prompt: {}", prompt);
                    }
                }

                println!(
                    "{} MCP injection complete in {:?} — restart the editor to see it in Plugin Manager.",
                    "✔".green().bold(),
                    start_time.elapsed()
                );
            }
            PluginCommands::List => {
                let binaries = mor_website_core::utils::mcp_install::list_installed_mcp_binaries();
                let plugin_ids =
                    mor_website_core::utils::mcp_install::list_registered_plugin_ids()
                        .map_err(|e| anyhow::anyhow!(e))?;
                let registry =
                    mor_website_core::utils::mcp_install::read_daemon_registry().map_err(|e| {
                        anyhow::anyhow!(e)
                    })?;

                println!("{} MCP plugin inventory", "➔".cyan());
                println!("  binaries ({}):", binaries.len());
                for name in binaries {
                    println!("    - {}", name);
                }
                println!("  config bridge plugins ({}):", plugin_ids.len());
                for id in plugin_ids {
                    println!("    - {}", id);
                }
                if let Some(servers) = registry.get("servers").and_then(|v| v.as_object()) {
                    println!("  daemon registry ({}):", servers.len());
                    for (key, entry) in servers {
                        let display = entry
                            .get("display_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(key);
                        println!("    - {} ({})", display, key);
                    }
                }
            }
        },
    }

    Ok(())
}

fn load_config(path: &PathBuf) -> Result<mor_website_core::config::ThemeConfig> {
    theme_config_from_path(path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load theme configuration from '{}': {}",
            path.display(),
            e
        )
    })
}
