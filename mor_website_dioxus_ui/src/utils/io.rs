use rfd::FileDialog;
use std::fs::File;
use std::io::Write;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

/// Save compiled `mor-theme.css` to a user-picked path.
pub fn save_css(css: &str) {
    if let Some(path) = FileDialog::new()
        .set_title("Export mor-theme.css")
        .set_file_name("mor-theme.css")
        .add_filter("CSS Stylesheet", &["css"])
        .save_file()
    {
        let _ = std::fs::write(path, css);
    }
}

pub fn save_toml(toml: &str) {
    if let Some(path) = FileDialog::new()
        .set_title("Save Theme Config As")
        .set_file_name("workspace.toml")
        .add_filter("TOML Config", &["toml"])
        .save_file()
    {
        let _ = std::fs::write(path, toml);
    }
}

pub fn load_toml() -> Option<String> {
    if let Some(path) = FileDialog::new()
        .set_title("Load Theme Config")
        .add_filter("TOML Config", &["toml"])
        .pick_file()
    {
        std::fs::read_to_string(path).ok()
    } else {
        None
    }
}

/// Bundle a compiled stylesheet + workspace TOML for backup/sharing.
pub fn export_bundle(css: &str, toml: &str) {
    if let Some(path) = FileDialog::new()
        .set_title("Export Theme Bundle")
        .set_file_name("mor_theme_bundle.zip")
        .add_filter("ZIP Archive", &["zip"])
        .save_file()
    {
        if let Ok(file) = File::create(path) {
            let mut zip = zip::ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

            let _ = zip.start_file("mor-theme.css", options);
            let _ = zip.write_all(css.as_bytes());

            let _ = zip.start_file("workspace.toml", options);
            let _ = zip.write_all(toml.as_bytes());

            let readme = "MorWebsite Theme Bundle\n\n\
1. Drop mor-theme.css (and workspace.toml) into your website project root.\n\
2. Link the stylesheet from every page:\n\
     <link rel=\"stylesheet\" href=\"/mor-theme.css\" />\n\
3. To keep editing: File → Open Website Folder… on that directory.\n\
   The editor loads workspace.toml automatically when present.\n\
\n\
See docs/SITE_CONTRACT.md for the modular site contract.\n";
            let _ = zip.start_file("README.txt", options);
            let _ = zip.write_all(readme.as_bytes());

            let _ = zip.finish();
        }
    }
}
