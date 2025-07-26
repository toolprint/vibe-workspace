use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Ensure directory exists, creating it if necessary
pub async fn ensure_directory_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        fs::create_dir_all(path)
            .await
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    } else if !path.is_dir() {
        anyhow::bail!("Path exists but is not a directory: {}", path.display());
    }

    Ok(())
}

/// Get relative path from base to target
pub fn get_relative_path<P: AsRef<Path>, Q: AsRef<Path>>(base: P, target: Q) -> PathBuf {
    let base = base.as_ref();
    let target = target.as_ref();

    match target.strip_prefix(base) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => target.to_path_buf(),
    }
}

/// Check if path is a subdirectory of base
pub fn is_subdirectory<P: AsRef<Path>, Q: AsRef<Path>>(base: P, path: Q) -> bool {
    let base = base.as_ref();
    let path = path.as_ref();

    path.starts_with(base)
}

/// Expand tilde in path
pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();

    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(path_str.strip_prefix("~/").unwrap());
            }
        } else if path_str == "~" {
            if let Some(home) = dirs::home_dir() {
                return home;
            }
        }
    }

    path.to_path_buf()
}

/// Check if file has any of the given extensions
pub fn has_extension<P: AsRef<Path>>(path: P, extensions: &[&str]) -> bool {
    let path = path.as_ref();

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        extensions
            .iter()
            .any(|&expected| ext.eq_ignore_ascii_case(expected))
    } else {
        false
    }
}

/// Get file size in bytes
pub async fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let metadata = fs::metadata(path.as_ref())
        .await
        .with_context(|| format!("Failed to get metadata for: {}", path.as_ref().display()))?;

    Ok(metadata.len())
}

/// Format file size in human-readable format
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_relative_path() {
        let base = Path::new("/home/user/workspace");
        let target = Path::new("/home/user/workspace/project");
        let relative = get_relative_path(base, target);
        assert_eq!(relative, Path::new("project"));

        let outside = Path::new("/other/path");
        let relative = get_relative_path(base, outside);
        assert_eq!(relative, Path::new("/other/path"));
    }

    #[test]
    fn test_is_subdirectory() {
        let base = Path::new("/home/user/workspace");
        let subdir = Path::new("/home/user/workspace/project");
        let outside = Path::new("/other/path");

        assert!(is_subdirectory(base, subdir));
        assert!(!is_subdirectory(base, outside));
        assert!(is_subdirectory(base, base)); // Same directory
    }

    #[test]
    fn test_expand_tilde() {
        // This test might fail in some environments where home dir is not available
        if dirs::home_dir().is_some() {
            let expanded = expand_tilde("~/test");
            assert!(expanded.to_string_lossy().contains("test"));
            assert!(!expanded.to_string_lossy().starts_with("~"));
        }

        // Test non-tilde paths
        let unchanged = expand_tilde("/absolute/path");
        assert_eq!(unchanged, Path::new("/absolute/path"));
    }

    #[test]
    fn test_has_extension() {
        assert!(has_extension("file.txt", &["txt", "md"]));
        assert!(has_extension("file.TXT", &["txt", "md"])); // Case insensitive
        assert!(!has_extension("file.txt", &["md", "rs"]));
        assert!(!has_extension("file", &["txt"])); // No extension
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[tokio::test]
    async fn test_ensure_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new_directory");

        // Should create directory
        let result = ensure_directory_exists(&new_dir).await;
        assert!(result.is_ok());
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());

        // Should succeed if directory already exists
        let result = ensure_directory_exists(&new_dir).await;
        assert!(result.is_ok());
    }
}
