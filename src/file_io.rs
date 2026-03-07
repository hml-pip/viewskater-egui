use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "bmp", "webp", "gif", "tiff", "tif", "qoi", "tga",
];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

pub fn enumerate_images(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        log::warn!("Failed to read directory: {}", dir.display());
        return Vec::new();
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let not_hidden = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('.'));
            not_hidden && is_supported_image(p)
        })
        .collect();

    paths.sort_by(|a, b| {
        natord::compare(
            &a.file_name().unwrap_or_default().to_string_lossy(),
            &b.file_name().unwrap_or_default().to_string_lossy(),
        )
    });

    log::info!("Found {} images in {}", paths.len(), dir.display());
    paths
}

/// Resolve a CLI path to a directory and an optional target filename.
/// If path is a file, returns its parent directory and the filename.
/// If path is a directory, returns it directly.
pub fn resolve_path(path: &Path) -> (PathBuf, Option<String>) {
    if path.is_file() {
        let dir = path.parent().unwrap_or(path).to_path_buf();
        let filename = path.file_name().map(|f| f.to_string_lossy().into_owned());
        (dir, filename)
    } else {
        (path.to_path_buf(), None)
    }
}
