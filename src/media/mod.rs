pub mod parser;
pub mod renamer;
pub mod scanner;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    Movie,
    TvEpisode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameStatus {
    /// Proposed path computed; waiting for user decision.
    Pending,
    /// User approved; waiting to be executed.
    Approved,
    /// User skipped.
    Skipped,
    /// Rename executed successfully.
    Done,
    /// Current path already matches the desired pattern.
    AlreadyCorrect,
    /// Waiting for API metadata response.
    LoadingMetadata,
    /// Rename failed (message included).
    Error(String),
}

impl RenameStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Skipped => "skipped",
            Self::Done => "done",
            Self::AlreadyCorrect => "ok",
            Self::LoadingMetadata => "loading",
            Self::Error(_) => "error",
        }
    }
}

/// Parsed/resolved metadata for a media file.
#[derive(Debug, Clone, Default)]
pub struct ParsedMetadata {
    // Movie fields
    pub title: Option<String>,
    pub year: Option<String>,
    // TV fields
    pub show: Option<String>,
    /// Year the show first aired — fetched from API for Plex-compliant directory names.
    pub show_year: Option<String>,
    pub season: Option<String>,
    pub episode: Option<String>,
    pub episode_title: Option<String>,
}

impl ParsedMetadata {
    /// True if all fields required for a movie rename are present.
    pub fn movie_complete(&self) -> bool {
        self.title.is_some() && self.year.is_some()
    }

    /// True if all fields required for a TV rename are present (episode title optional).
    pub fn tv_complete(&self) -> bool {
        self.show.is_some() && self.season.is_some() && self.episode.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct MediaFile {
    pub path: PathBuf,
    pub media_type: MediaType,
    /// Metadata extracted directly from the filename.
    pub parsed_metadata: Option<ParsedMetadata>,
    /// Final metadata (may be enriched by API).
    pub resolved_metadata: Option<ParsedMetadata>,
    /// The proposed new absolute path after rename.
    pub proposed_path: Option<PathBuf>,
    pub status: RenameStatus,
}

impl MediaFile {
    pub fn new(path: PathBuf, media_type: MediaType) -> Self {
        Self {
            path,
            media_type,
            parsed_metadata: None,
            resolved_metadata: None,
            proposed_path: None,
            status: RenameStatus::Pending,
        }
    }

    pub fn display_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
    }

    /// True if the proposed path differs from the current path.
    pub fn needs_rename(&self) -> bool {
        match &self.proposed_path {
            Some(proposed) => proposed != &self.path,
            None => false,
        }
    }

    /// Returns the effective metadata: resolved first, then parsed.
    pub fn effective_metadata(&self) -> Option<&ParsedMetadata> {
        self.resolved_metadata.as_ref().or(self.parsed_metadata.as_ref())
    }
}
