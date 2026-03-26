use std::path::{Path, PathBuf};

/// Generate SRT output path following Plex/Jellyfin naming convention.
/// Example: /media/Movie.mkv + "fr" → /media/Movie.fr.srt
pub fn srt_path(media_path: &Path, lang: &str) -> PathBuf {
    let stem = media_path.file_stem().unwrap_or_default();
    let parent = media_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}.{}.srt", stem.to_string_lossy(), lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_path() {
        let p = srt_path(Path::new("/media/Movie (2024).mkv"), "fr");
        assert_eq!(p, PathBuf::from("/media/Movie (2024).fr.srt"));
    }

    #[test]
    fn test_srt_path_series() {
        let p = srt_path(Path::new("/tv/Show S01E01.mkv"), "en");
        assert_eq!(p, PathBuf::from("/tv/Show S01E01.en.srt"));
    }
}
