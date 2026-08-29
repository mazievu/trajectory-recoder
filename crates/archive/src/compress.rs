use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::{Archive as TarArchive, Builder as TarBuilder};

/// Compress a directory into a TAR archive compressed with Zstandard.
pub fn create_tar_zstd_archive(
    source_dir: impl AsRef<Path>,
    output_archive_path: impl AsRef<Path>,
    zstd_level: i32,
) -> std::io::Result<(u64, u64, Vec<String>)> {
    let src = source_dir.as_ref();
    let dst = output_archive_path.as_ref();

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let out_file = File::create(dst)?;
    let zstd_encoder = zstd::stream::Encoder::new(out_file, zstd_level)?;
    let mut tar_builder = TarBuilder::new(zstd_encoder);

    let mut uncompressed_size = 0u64;
    let mut file_list = Vec::new();

    // Recursively add all files in directory
    for entry in walkdir(src)? {
        let rel_path = entry.strip_prefix(src).unwrap_or(&entry);
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");

        if entry.is_file() {
            let metadata = std::fs::metadata(&entry)?;
            uncompressed_size += metadata.len();
            let mut file = File::open(&entry)?;
            tar_builder.append_file(&rel_str, &mut file)?;
            file_list.push(rel_str);
        }
    }

    let zstd_encoder = tar_builder.into_inner()?;
    let mut out_file = zstd_encoder.finish()?;
    out_file.flush()?;

    let compressed_size = std::fs::metadata(dst)?.len();
    Ok((uncompressed_size, compressed_size, file_list))
}

/// Extract a TAR.Zstd archive to a target directory.
pub fn extract_tar_zstd_archive(
    archive_path: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
) -> std::io::Result<()> {
    let archive_file = File::open(archive_path.as_ref())?;
    let zstd_decoder = zstd::stream::Decoder::new(archive_file)?;
    let mut tar_archive = TarArchive::new(zstd_decoder);
    tar_archive.unpack(target_dir.as_ref())?;
    Ok(())
}

fn walkdir(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tar_zstd_compression_and_extraction() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let extract_dir = tempdir().unwrap();

        // Create sample session files
        std::fs::write(src_dir.path().join("events.raw.ndjson"), "{\"event\":1}\n{\"event\":2}\n").unwrap();
        std::fs::write(src_dir.path().join("session.db"), "SQLITE_HEADER_MOCK_DATA").unwrap();

        let archive_path = out_dir.path().join("session.tar.zst");
        let (uncompressed, compressed, files) = create_tar_zstd_archive(src_dir.path(), &archive_path, 3).unwrap();

        assert!(uncompressed > 0);
        assert!(compressed > 0);
        assert_eq!(files.len(), 2);
        assert!(archive_path.exists());

        // Extract
        extract_tar_zstd_archive(&archive_path, extract_dir.path()).unwrap();
        assert!(extract_dir.path().join("events.raw.ndjson").exists());
        assert!(extract_dir.path().join("session.db").exists());
    }
}
