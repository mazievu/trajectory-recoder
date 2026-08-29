use std::path::Path;

/// Check if a file path is a known temporary, swap, or system noise file that should be ignored.
pub fn is_noise_file(path: &str) -> bool {
    let p = Path::new(path);
    let file_name = match p.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Ignore Windows desktop.ini, Thumbs.db
    if file_name.eq_ignore_ascii_case("desktop.ini")
        || file_name.eq_ignore_ascii_case("thumbs.db")
        || file_name.eq_ignore_ascii_case(".ds_store")
    {
        return true;
    }

    // Ignore Office temporary lock files (e.g. ~$Document.docx)
    if file_name.starts_with("~$") {
        return true;
    }

    // Ignore temp extensions
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        if ext_lower == "tmp"
            || ext_lower == "temp"
            || ext_lower == "crdownload"
            || ext_lower == "part"
            || ext_lower == "swp"
            || ext_lower == "lock"
        {
            return true;
        }
    }

    false
}
