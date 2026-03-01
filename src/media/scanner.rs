use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::Config;
use crate::media::parser::{MovieParser, TvParser};
use crate::media::renamer::compute_proposed_path;
use crate::media::{MediaFile, MediaType, RenameStatus};

/// Video file extensions we recognise.
const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "m4v", "wmv", "flv", "ts", "m2ts", "vob",
];

/// Scan all configured media roots and return a list of `MediaFile`s.
pub fn scan_all(config: &Config) -> Result<Vec<MediaFile>> {
    let mut files = Vec::new();

    let movie_parser = MovieParser::new(&config.patterns.movies)?;
    let tv_parser = TvParser::new(&config.patterns.tv_shows)?;

    if let Some(ref root) = config.roots.movies {
        if root.exists() {
            let mut movies = scan_directory(root, MediaType::Movie, &movie_parser, config)?;
            files.append(&mut movies);
        } else {
            tracing::warn!("Movies root does not exist: {}", root.display());
        }
    }

    if let Some(ref root) = config.roots.tv_shows {
        if root.exists() {
            let mut tv = scan_directory(root, MediaType::TvEpisode, &tv_parser, config)?;
            files.append(&mut tv);
        } else {
            tracing::warn!("TV shows root does not exist: {}", root.display());
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn scan_directory(
    root: &Path,
    media_type: MediaType,
    parser: &dyn FilenameParser,
    config: &Config,
) -> Result<Vec<MediaFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if !is_video_file(path) {
            continue;
        }

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let parsed = parser.parse(filename);

        let mut file = MediaFile::new(path.to_path_buf(), media_type.clone());
        file.parsed_metadata = parsed;

        // For TV episodes with API enabled, always request API enrichment so we
        // can fetch show_year for Plex-compliant directory names.  Pre-compute a
        // preliminary path (empty parens cleaned up) for the preview while loading.
        if media_type == MediaType::TvEpisode && config.api_enabled() {
            if let Ok(Some(proposed)) = compute_proposed_path(&file, config) {
                file.proposed_path = Some(proposed);
            }
            file.status = RenameStatus::LoadingMetadata;
        } else {
            // Movies (and TV when API is disabled): compute path immediately.
            match compute_proposed_path(&file, config) {
                Ok(Some(proposed)) => {
                    if proposed == file.path {
                        file.status = RenameStatus::AlreadyCorrect;
                    } else {
                        file.proposed_path = Some(proposed);
                        file.status = RenameStatus::Pending;
                    }
                }
                Ok(None) => {
                    file.status = RenameStatus::LoadingMetadata;
                }
                Err(e) => {
                    file.status = RenameStatus::Error(e.to_string());
                }
            }
        }

        files.push(file);
    }

    Ok(files)
}

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Trait object so `scan_directory` works with both parser types.
trait FilenameParser {
    fn parse(&self, filename: &str) -> Option<crate::media::ParsedMetadata>;
}

impl FilenameParser for MovieParser {
    fn parse(&self, filename: &str) -> Option<crate::media::ParsedMetadata> {
        MovieParser::parse(self, filename)
    }
}

impl FilenameParser for TvParser {
    fn parse(&self, filename: &str) -> Option<crate::media::ParsedMetadata> {
        TvParser::parse(self, filename)
    }
}
