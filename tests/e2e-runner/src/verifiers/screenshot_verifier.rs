use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotValidationReport {
    pub total_screenshots_found: usize,
    pub valid_webp_headers: usize,
    pub corrupted_images: Vec<String>,
    pub missing_expected_files: Vec<String>,
}

pub struct ScreenshotVerifier;

impl ScreenshotVerifier {
    pub fn is_valid_webp_header(bytes: &[u8]) -> bool {
        if bytes.len() < 12 {
            return false;
        }
        // RIFF header: "RIFF" (0..4) and "WEBP" (8..12)
        &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
    }

    pub fn verify_directory(
        screenshots_dir: &Path,
        expected_filenames: &[String],
    ) -> Result<ScreenshotValidationReport, String> {
        let mut report = ScreenshotValidationReport {
            total_screenshots_found: 0,
            valid_webp_headers: 0,
            corrupted_images: Vec::new(),
            missing_expected_files: Vec::new(),
        };

        if !screenshots_dir.exists() {
            if expected_filenames.is_empty() {
                return Ok(report);
            } else {
                report.missing_expected_files = expected_filenames.to_vec();
                return Ok(report);
            }
        }

        let entries = fs::read_dir(screenshots_dir).map_err(|e| {
            format!(
                "Failed to read screenshots dir {}: {}",
                screenshots_dir.display(),
                e
            )
        })?;

        let mut existing_files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                report.total_screenshots_found += 1;
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    existing_files.push(fname.to_string());
                }

                match fs::read(&path) {
                    Ok(bytes) => {
                        if Self::is_valid_webp_header(&bytes) {
                            report.valid_webp_headers += 1;
                        } else {
                            report.corrupted_images.push(format!(
                                "{}: Invalid WebP RIFF/WEBP header or size < 12 bytes",
                                path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        report.corrupted_images.push(format!(
                            "{}: I/O read error: {}",
                            path.display(),
                            e
                        ));
                    }
                }
            }
        }

        for expected in expected_filenames {
            if !existing_files.contains(expected) {
                report.missing_expected_files.push(expected.clone());
            }
        }

        Ok(report)
    }

    pub fn verify_bounding_box_within_bounds(
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        screen_w: f64,
        screen_h: f64,
    ) -> bool {
        x >= 0.0 && y >= 0.0 && (x + w) <= screen_w && (y + h) <= screen_h
    }
}
