use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ArchiveValidationReport {
    pub is_valid_zstd: bool,
    pub unpacked_entries: Vec<String>,
    pub manifest_found: bool,
    pub raw_ndjson_found: bool,
    pub normalized_ndjson_found: bool,
    pub sqlite_found: bool,
    pub total_uncompressed_bytes: u64,
    pub archive_sha256: String,
    pub errors: Vec<String>,
}

pub struct ArchiveVerifier;

impl ArchiveVerifier {
    pub fn verify_tar_zst_archive(path: &Path) -> Result<ArchiveValidationReport, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open archive at {}: {}", path.display(), e))?;

        // 1. Calculate overall archive file SHA-256
        let mut hasher = Sha256::new();
        let mut file_for_hash = File::open(path).map_err(|e| e.to_string())?;
        let mut buffer = [0u8; 65536];
        loop {
            let n = file_for_hash.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let archive_sha256 = format!("{:x}", hasher.finalize());

        let mut report = ArchiveValidationReport {
            is_valid_zstd: false,
            unpacked_entries: Vec::new(),
            manifest_found: false,
            raw_ndjson_found: false,
            normalized_ndjson_found: false,
            sqlite_found: false,
            total_uncompressed_bytes: 0,
            archive_sha256,
            errors: Vec::new(),
        };

        // 2. Decode Zstandard stream
        let zstd_decoder = match zstd::Decoder::new(file) {
            Ok(d) => {
                report.is_valid_zstd = true;
                d
            }
            Err(e) => {
                report.errors.push(format!("Zstd decompression init failed: {}", e));
                return Ok(report);
            }
        };

        // 3. Unpack TAR entries
        let mut tar_archive = tar::Archive::new(zstd_decoder);
        let entries = match tar_archive.entries() {
            Ok(e) => e,
            Err(e) => {
                report.errors.push(format!("TAR header read failed: {}", e));
                return Ok(report);
            }
        };

        for entry_res in entries {
            match entry_res {
                Ok(entry) => {
                    let path_name = match entry.path() {
                        Ok(p) => p.to_string_lossy().to_string(),
                        Err(e) => {
                            report.errors.push(format!("Corrupt TAR entry path: {}", e));
                            continue;
                        }
                    };
                    report.total_uncompressed_bytes += entry.size();
                    report.unpacked_entries.push(path_name.clone());

                    if path_name.ends_with("manifest.json") {
                        report.manifest_found = true;
                    }
                    if path_name.ends_with("events.raw.ndjson") {
                        report.raw_ndjson_found = true;
                    }
                    if path_name.ends_with("events.normalized.ndjson") {
                        report.normalized_ndjson_found = true;
                    }
                    if path_name.ends_with("index.sqlite") {
                        report.sqlite_found = true;
                    }
                }
                Err(e) => {
                    report.errors.push(format!("TAR entry read error: {}", e));
                }
            }
        }

        Ok(report)
    }
}
