/// Structural data for font mapping. No heap strings allowed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontPreset {
    pub name: &'static str,
    pub css_stack: &'static str,
    pub google_font_name: Option<&'static str>,
    pub category: &'static str,
}

/// Static catalog matching your preset aesthetics. Fast lookup, zero heap bloat.
pub const FONT_REGISTRY: &[FontPreset] = &[
    // --- Web 1.0 & System Safe ---
    FontPreset {
        name: "Times New Roman",
        css_stack: "'Times New Roman', Times, serif",
        google_font_name: None,
        category: "Web 1.0 / Classic",
    },
    FontPreset {
        name: "Arial",
        css_stack: "Arial, sans-serif",
        google_font_name: None,
        category: "Web 1.0 / Classic",
    },
    FontPreset {
        name: "Courier New",
        css_stack: "'Courier New', Courier, monospace",
        google_font_name: None,
        category: "Web 1.0 / Classic",
    },
    FontPreset {
        name: "Comic Sans",
        css_stack: "'Comic Sans MS', 'Comic Sans', cursive",
        google_font_name: None,
        category: "Web 1.0 / Classic",
    },
    FontPreset {
        name: "Impact",
        css_stack: "Impact, Haettenschweiler, 'Arial Narrow Bold', sans-serif",
        google_font_name: None,
        category: "Web 1.0 / Classic",
    },
    // --- Modern Clean Sans ---
    FontPreset {
        name: "Inter",
        css_stack: "'Inter', sans-serif",
        google_font_name: Some("Inter"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Roboto",
        css_stack: "'Roboto', sans-serif",
        google_font_name: Some("Roboto"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Montserrat",
        css_stack: "'Montserrat', sans-serif",
        google_font_name: Some("Montserrat"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Open Sans",
        css_stack: "'Open Sans', sans-serif",
        google_font_name: Some("Open Sans"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Poppins",
        css_stack: "'Poppins', sans-serif",
        google_font_name: Some("Poppins"),
        category: "Modern Sans",
    },
    // --- Elegant Serifs ---
    FontPreset {
        name: "Merriweather",
        css_stack: "'Merriweather', serif",
        google_font_name: Some("Merriweather"),
        category: "Serif",
    },
    FontPreset {
        name: "Playfair Display",
        css_stack: "'Playfair Display', serif",
        google_font_name: Some("Playfair Display"),
        category: "Serif",
    },
    FontPreset {
        name: "Lora",
        css_stack: "'Lora', serif",
        google_font_name: Some("Lora"),
        category: "Serif",
    },
    FontPreset {
        name: "IM Fell English",
        css_stack: "'IM Fell English', serif",
        google_font_name: Some("IM Fell English"),
        category: "Serif / Old World",
    },
    FontPreset {
        name: "Cinzel",
        css_stack: "'Cinzel', serif",
        google_font_name: Some("Cinzel"),
        category: "Serif / Display",
    },
    // --- Cyberpunk & Display ---
    FontPreset {
        name: "Orbitron",
        css_stack: "'Orbitron', sans-serif",
        google_font_name: Some("Orbitron"),
        category: "Display / Sci-Fi",
    },
    FontPreset {
        name: "Press Start 2P",
        css_stack: "'Press Start 2P', cursive",
        google_font_name: Some("Press Start 2P"),
        category: "Display / Retro",
    },
    FontPreset {
        name: "Righteous",
        css_stack: "'Righteous', cursive",
        google_font_name: Some("Righteous"),
        category: "Display / Chunky",
    },
    FontPreset {
        name: "Bebas Neue",
        css_stack: "'Bebas Neue', sans-serif",
        google_font_name: Some("Bebas Neue"),
        category: "Display / Tall",
    },
    // --- System UI stacks (no webfont download) ---
    FontPreset {
        name: "System UI",
        css_stack: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        google_font_name: None,
        category: "System UI",
    },
    FontPreset {
        name: "Native Serif",
        css_stack: "ui-serif, Georgia, 'Times New Roman', Times, serif",
        google_font_name: None,
        category: "System UI",
    },
    FontPreset {
        name: "Native Mono",
        css_stack: "ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace",
        google_font_name: None,
        category: "System UI",
    },
    // --- Expanded modern sans (website-editor freedom) ---
    FontPreset {
        name: "DM Sans",
        css_stack: "'DM Sans', system-ui, sans-serif",
        google_font_name: Some("DM Sans"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Work Sans",
        css_stack: "'Work Sans', system-ui, sans-serif",
        google_font_name: Some("Work Sans"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Source Sans 3",
        css_stack: "'Source Sans 3', system-ui, sans-serif",
        google_font_name: Some("Source Sans 3"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Nunito",
        css_stack: "'Nunito', system-ui, sans-serif",
        google_font_name: Some("Nunito"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Manrope",
        css_stack: "'Manrope', system-ui, sans-serif",
        google_font_name: Some("Manrope"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Space Grotesk",
        css_stack: "'Space Grotesk', system-ui, sans-serif",
        google_font_name: Some("Space Grotesk"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "IBM Plex Sans",
        css_stack: "'IBM Plex Sans', system-ui, sans-serif",
        google_font_name: Some("IBM Plex Sans"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Outfit",
        css_stack: "'Outfit', system-ui, sans-serif",
        google_font_name: Some("Outfit"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Figtree",
        css_stack: "'Figtree', system-ui, sans-serif",
        google_font_name: Some("Figtree"),
        category: "Modern Sans",
    },
    FontPreset {
        name: "Plus Jakarta Sans",
        css_stack: "'Plus Jakarta Sans', system-ui, sans-serif",
        google_font_name: Some("Plus Jakarta Sans"),
        category: "Modern Sans",
    },
    // --- More serifs ---
    FontPreset {
        name: "Source Serif 4",
        css_stack: "'Source Serif 4', Georgia, serif",
        google_font_name: Some("Source Serif 4"),
        category: "Serif",
    },
    FontPreset {
        name: "Literata",
        css_stack: "'Literata', Georgia, serif",
        google_font_name: Some("Literata"),
        category: "Serif",
    },
    FontPreset {
        name: "Crimson Pro",
        css_stack: "'Crimson Pro', Georgia, serif",
        google_font_name: Some("Crimson Pro"),
        category: "Serif",
    },
    FontPreset {
        name: "Libre Baskerville",
        css_stack: "'Libre Baskerville', Georgia, serif",
        google_font_name: Some("Libre Baskerville"),
        category: "Serif",
    },
    FontPreset {
        name: "Instrument Serif",
        css_stack: "'Instrument Serif', Georgia, serif",
        google_font_name: Some("Instrument Serif"),
        category: "Serif / Display",
    },
    FontPreset {
        name: "Fraunces",
        css_stack: "'Fraunces', Georgia, serif",
        google_font_name: Some("Fraunces"),
        category: "Serif / Display",
    },
    // --- Handwriting / display extras ---
    FontPreset {
        name: "Caveat",
        css_stack: "'Caveat', cursive",
        google_font_name: Some("Caveat"),
        category: "Handwriting",
    },
    FontPreset {
        name: "Pacifico",
        css_stack: "'Pacifico', cursive",
        google_font_name: Some("Pacifico"),
        category: "Handwriting",
    },
    FontPreset {
        name: "Syne",
        css_stack: "'Syne', system-ui, sans-serif",
        google_font_name: Some("Syne"),
        category: "Display / Tall",
    },
];

pub const MONO_FONT_REGISTRY: &[FontPreset] = &[
    FontPreset {
        name: "JetBrains Mono",
        css_stack: "'JetBrains Mono', monospace",
        google_font_name: Some("JetBrains Mono"),
        category: "Monospace",
    },
    FontPreset {
        name: "Fira Code",
        css_stack: "'Fira Code', monospace",
        google_font_name: Some("Fira Code"),
        category: "Monospace",
    },
    FontPreset {
        name: "Space Mono",
        css_stack: "'Space Mono', monospace",
        google_font_name: Some("Space Mono"),
        category: "Monospace",
    },
    FontPreset {
        name: "Inconsolata",
        css_stack: "'Inconsolata', monospace",
        google_font_name: Some("Inconsolata"),
        category: "Monospace",
    },
    FontPreset {
        name: "Source Code Pro",
        css_stack: "'Source Code Pro', monospace",
        google_font_name: Some("Source Code Pro"),
        category: "Monospace",
    },
    FontPreset {
        name: "IBM Plex Mono",
        css_stack: "'IBM Plex Mono', ui-monospace, monospace",
        google_font_name: Some("IBM Plex Mono"),
        category: "Monospace",
    },
    FontPreset {
        name: "Roboto Mono",
        css_stack: "'Roboto Mono', ui-monospace, monospace",
        google_font_name: Some("Roboto Mono"),
        category: "Monospace",
    },
    FontPreset {
        name: "Ubuntu Mono",
        css_stack: "'Ubuntu Mono', ui-monospace, monospace",
        google_font_name: Some("Ubuntu Mono"),
        category: "Monospace",
    },
    FontPreset {
        name: "Courier New",
        css_stack: "'Courier New', Courier, monospace",
        google_font_name: None,
        category: "System Safe",
    },
    FontPreset {
        name: "Native Mono",
        css_stack: "ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, monospace",
        google_font_name: None,
        category: "System UI",
    },
];

/// Primary family name from a CSS font stack (text before the first comma).
pub fn primary_font_from_stack(stack: &str) -> &str {
    stack
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
}

/// True when the primary font does not require a Google Fonts stylesheet.
pub fn is_system_safe_font(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return true;
    }

    const SYSTEM_SAFE_FONTS: &[&str] = &[
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
        "-apple-system",
        "blinkmacsystemfont",
        "segoe ui",
        "helvetica",
        "helvetica neue",
        "arial",
        "verdana",
        "tahoma",
        "trebuchet ms",
        "georgia",
        "times",
        "times new roman",
        "courier",
        "courier new",
        "lucida console",
        "lucida sans unicode",
        "comic sans ms",
        "impact",
    ];

    if SYSTEM_SAFE_FONTS
        .iter()
        .any(|&f| f.eq_ignore_ascii_case(name))
    {
        return true;
    }

    for font in FONT_REGISTRY.iter().chain(MONO_FONT_REGISTRY.iter()) {
        if font.name.eq_ignore_ascii_case(name) {
            return font.google_font_name.is_none();
        }
    }

    false
}

fn implies_serif(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("serif")
        || lower.contains("roman")
        || lower.contains("garamond")
        || lower.contains("georgia")
        || lower.contains("palatino")
        || lower.contains("times")
        || lower.contains("baskerville")
        || lower.contains("caslon")
        || lower.contains("bodoni")
}

pub fn resolve_font_stack_with_fallback(raw_input: &str, is_mono: bool) -> String {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return if is_mono {
            "ui-monospace, 'Courier New', Courier, monospace".to_string()
        } else {
            "system-ui, -apple-system, sans-serif".to_string()
        };
    }

    let registry = if is_mono {
        MONO_FONT_REGISTRY
    } else {
        FONT_REGISTRY
    };

    for font in registry {
        if font.name.eq_ignore_ascii_case(trimmed) || font.css_stack.eq_ignore_ascii_case(trimmed) {
            return font.css_stack.to_string();
        }
    }

    if trimmed.contains(',') {
        return trimmed.to_string();
    }

    let generic = if is_mono {
        "monospace"
    } else if implies_serif(trimmed) {
        "serif"
    } else {
        "sans-serif"
    };

    if trimmed.contains(' ') {
        format!("\"{}\", {}", trimmed, generic)
    } else {
        format!("{}, {}", trimmed, generic)
    }
}

fn google_family_query(primary: &str) -> String {
    for font in FONT_REGISTRY.iter().chain(MONO_FONT_REGISTRY.iter()) {
        if font.name.eq_ignore_ascii_case(primary) {
            if let Some(google_name) = font.google_font_name {
                return format!("{}:wght@400;500;600;700", google_name.replace(' ', "+"));
            }
            break;
        }
    }
    format!("{}:wght@400;500;600;700", primary.replace(' ', "+"))
}

/// A `fonts.googleapis.com/css2?family=…` URL covering every Google font in
/// both registries. Used by the editor's Typography panel so the dropdown
/// previews and live preview render each preset font in its real typeface.
/// Returns an empty string if no Google fonts are registered.
pub fn all_registry_google_fonts_url() -> String {
    let mut families: Vec<String> = Vec::new();
    for font in FONT_REGISTRY.iter().chain(MONO_FONT_REGISTRY.iter()) {
        if let Some(name) = font.google_font_name {
            let query = format!("{}:wght@400;500;600;700", name.replace(' ', "+"));
            if !families.iter().any(|f| f == &query) {
                families.push(query);
            }
        }
    }
    if families.is_empty() {
        return String::new();
    }
    format!(
        "https://fonts.googleapis.com/css2?family={}&display=swap",
        families.join("&family=")
    )
}

/// Webfont host for hosted families (not system / not self-hosted `@font-face`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontProvider {
    /// fonts.googleapis.com (classic)
    Google,
    /// fonts.bunny.net — same catalog, no Google tracking (recommended default)
    Bunny,
    /// Do not emit remote webfont links; use system fonts or site-owned `@font-face`.
    None,
}

impl FontProvider {
    pub fn from_config(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "bunny" | "fonts.bunny" | "fonts.bunny.net" => FontProvider::Bunny,
            "none" | "off" | "self" | "self-host" | "local" => FontProvider::None,
            _ => FontProvider::Google,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FontProvider::Google => "google",
            FontProvider::Bunny => "bunny",
            FontProvider::None => "none",
        }
    }
}

/// Collect unique non-system primary families from CSS stacks.
fn collect_webfont_families(font_stacks: &[&str]) -> Vec<String> {
    let mut families: Vec<String> = Vec::new();
    for stack in font_stacks {
        let primary = primary_font_from_stack(stack);
        if primary.is_empty() || is_system_safe_font(primary) {
            continue;
        }
        // Skip stacks that already embed url() (custom self-host experiment).
        if stack.contains("url(") {
            continue;
        }
        let query = google_family_query(primary);
        if !families.iter().any(|f| f == &query) {
            families.push(query);
        }
    }
    families
}

/// Absolute stylesheet URL for the active webfont host, or empty if none.
pub fn webfont_stylesheet_href(font_stacks: &[&str], provider: FontProvider) -> String {
    if matches!(provider, FontProvider::None) {
        return String::new();
    }
    let families = collect_webfont_families(font_stacks);
    if families.is_empty() {
        return String::new();
    }
    match provider {
        FontProvider::Google => {
            format!(
                "https://fonts.googleapis.com/css2?family={}&display=swap",
                families.join("&family=")
            )
        }
        FontProvider::Bunny => {
            // Bunny: family=inter:400,500,600,700|playfair-display:400,700
            let bunny: Vec<String> = families
                .iter()
                .map(|q| {
                    // "Inter:wght@400;500;600;700" → "inter:400,500,600,700"
                    let (name, weights) = q.split_once(":wght@").unwrap_or((q.as_str(), "400;700"));
                    let slug = name.replace('+', "-").to_ascii_lowercase();
                    let w = weights.replace(';', ",");
                    format!("{slug}:{w}")
                })
                .collect();
            format!(
                "https://fonts.bunny.net/css?family={}&display=swap",
                bunny.join("|")
            )
        }
        FontProvider::None => String::new(),
    }
}

/// Build webfont `<link>` tags for HTML (preview / legacy Blogger path).
pub fn build_google_font_imports(font_stacks: &[&str]) -> String {
    build_webfont_link_tag(font_stacks, FontProvider::Google)
}

/// HTML `<link rel="stylesheet">` for the chosen provider (escaped `&` for XML/HTML).
pub fn build_webfont_link_tag(font_stacks: &[&str], provider: FontProvider) -> String {
    let href = webfont_stylesheet_href(font_stacks, provider);
    if href.is_empty() {
        return String::new();
    }
    // Escape & for safe insertion into HTML/XML attribute context.
    let safe = href.replace('&', "&amp;");
    format!("<link rel=\"stylesheet\" href=\"{safe}\"/>")
}

/// CSS `@import` block for `mor-theme.css` (must be the first rules in the file).
pub fn build_webfont_css_import(font_stacks: &[&str], provider: FontProvider) -> String {
    let href = webfont_stylesheet_href(font_stacks, provider);
    if href.is_empty() {
        return String::new();
    }
    format!("@import url(\"{href}\");\n\n")
}

/// Stacks commonly used from a ThemeConfig for webfont resolution.
pub fn typography_font_stacks(
    body: &str,
    heading: &str,
    mono: &str,
) -> [String; 3] {
    [
        resolve_font_stack_with_fallback(body, false),
        if heading.trim().is_empty() {
            resolve_font_stack_with_fallback(body, false)
        } else {
            resolve_font_stack_with_fallback(heading, false)
        },
        resolve_font_stack_with_fallback(mono, true),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemeConfig;

    #[test]
    fn skips_system_safe_fonts() {
        assert!(is_system_safe_font("Courier New"));
        assert!(is_system_safe_font("system-ui"));
        assert!(is_system_safe_font("serif"));
        assert!(!is_system_safe_font("Inter"));
    }

    #[test]
    fn builds_weighted_google_link() {
        let link = build_google_font_imports(&["Inter", "'Playfair Display', serif"]);
        assert!(link.contains("rel=\"stylesheet\""));
        assert!(link.contains("Inter:wght@400;500;600;700"));
        assert!(link.contains("Playfair+Display:wght@400;500;600;700"));
        assert!(link.contains("&amp;display=swap"));
    }

    #[test]
    fn bunny_provider_uses_bunny_host() {
        let href = webfont_stylesheet_href(&["Inter", "Playfair Display"], FontProvider::Bunny);
        assert!(href.contains("fonts.bunny.net"));
        assert!(href.contains("inter:"));
        assert!(!href.contains("googleapis"));
    }

    #[test]
    fn none_provider_emits_nothing() {
        assert!(webfont_stylesheet_href(&["Inter"], FontProvider::None).is_empty());
        assert!(build_webfont_css_import(&["Inter"], FontProvider::None).is_empty());
    }

    #[test]
    fn arbitrary_custom_name_gets_stack_and_webfont() {
        assert_eq!(
            resolve_font_stack_with_fallback("Raleway", false),
            "Raleway, sans-serif"
        );
        let href = webfont_stylesheet_href(&["Raleway, sans-serif"], FontProvider::Google);
        assert!(href.contains("Raleway"));
    }

    #[test]
    fn full_custom_stack_passes_through() {
        let stack = "'My Brand', 'Helvetica Neue', system-ui, sans-serif";
        assert_eq!(resolve_font_stack_with_fallback(stack, false), stack);
    }

    #[test]
    fn system_stack_produces_no_link() {
        let link = build_google_font_imports(&[
            "system-ui, -apple-system, sans-serif",
            "'Courier New', Courier, monospace",
        ]);
        assert!(link.is_empty());
    }

    #[test]
    fn exported_theme_injects_font_link_in_head() {
        let mut config = ThemeConfig::default();
        config.typography.body_font_stack = "Inter".to_string();
        let xml = crate::render::render_theme(&config, &std::collections::HashMap::new());
        assert!(xml.contains("fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700"));
    }

    #[test]
    fn empty_stacks_fall_back_to_native() {
        assert_eq!(
            resolve_font_stack_with_fallback("", false),
            "system-ui, -apple-system, sans-serif"
        );
        assert_eq!(
            resolve_font_stack_with_fallback("   ", false),
            "system-ui, -apple-system, sans-serif"
        );
        assert_eq!(
            resolve_font_stack_with_fallback("", true),
            "ui-monospace, 'Courier New', Courier, monospace"
        );
    }

    #[test]
    fn custom_spaced_font_uses_double_quotes() {
        assert_eq!(
            resolve_font_stack_with_fallback("My Custom Font", false),
            "\"My Custom Font\", sans-serif"
        );
        assert_eq!(
            resolve_font_stack_with_fallback("Old Garamond Text", false),
            "\"Old Garamond Text\", serif"
        );
        assert_eq!(
            resolve_font_stack_with_fallback("Code Thing", true),
            "\"Code Thing\", monospace"
        );
    }

    #[test]
    fn single_word_custom_font_no_quotes() {
        assert_eq!(
            resolve_font_stack_with_fallback("Raleway", false),
            "Raleway, sans-serif"
        );
    }

    #[test]
    fn serif_name_detection() {
        assert_eq!(
            resolve_font_stack_with_fallback("MySerif", false),
            "MySerif, serif"
        );
        assert_eq!(
            resolve_font_stack_with_fallback("BookRoman", false),
            "BookRoman, serif"
        );
    }

    #[test]
    fn css_builder_maps_font_variables() {
        let mut config = ThemeConfig::default();
        config.typography.body_font_stack = "Inter".to_string();
        config.typography.heading_font_stack = "Playfair Display".to_string();
        config.typography.mono_font_stack = "Courier New".to_string();
        let css = crate::render::css_builder::build_master_css(&[], &config);
        assert!(css.contains("--font-body: 'Inter', sans-serif;"));
        assert!(css.contains("--font-heading: 'Playfair Display', serif;"));
        assert!(css.contains("--font-mono: 'Courier New', Courier, monospace;"));
        assert!(css.contains("--font-size-base: 16px;"));
        assert!(css.contains("--line-height-body: 1.6;"));
        assert!(css.contains("--heading-weight: 600;"));
    }
}
