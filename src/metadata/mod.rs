pub mod tmdb;
pub mod tvdb;

use anyhow::Result;

use crate::config::Config;
use crate::media::{MediaFile, MediaType, ParsedMetadata};
use crate::metadata::tmdb::TmdbClient;
use crate::metadata::tvdb::TvdbClient;

pub struct MetadataResolver {
    tmdb: Option<TmdbClient>,
    tvdb: Option<TvdbClient>,
}

impl MetadataResolver {
    pub fn new(config: &Config) -> Self {
        let tmdb = if !config.api.tmdb_api_key.is_empty() {
            Some(TmdbClient::new(config.api.tmdb_api_key.clone()))
        } else {
            None
        };

        let tvdb = if !config.api.tvdb_api_key.is_empty() {
            Some(TvdbClient::new(config.api.tvdb_api_key.clone()))
        } else {
            None
        };

        Self { tmdb, tvdb }
    }

    /// Resolve complete metadata for a file, calling the API if needed.
    pub async fn resolve(&self, file: &MediaFile) -> Result<Option<ParsedMetadata>> {
        let parsed = match file.parsed_metadata.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(None),
        };

        match file.media_type {
            MediaType::Movie => self.resolve_movie(parsed).await,
            MediaType::TvEpisode => self.resolve_tv(parsed).await,
        }
    }

    async fn resolve_movie(&self, mut meta: ParsedMetadata) -> Result<Option<ParsedMetadata>> {
        // If we already have both title and year, return immediately.
        if meta.movie_complete() {
            return Ok(Some(meta));
        }

        if let Some(tmdb) = &self.tmdb {
            if let Ok(Some(api_meta)) = tmdb
                .search_movie(
                    meta.title.as_deref().unwrap_or(""),
                    meta.year.as_deref(),
                )
                .await
            {
                // Fill in any missing fields from API result.
                if meta.title.is_none() {
                    meta.title = api_meta.title;
                }
                if meta.year.is_none() {
                    meta.year = api_meta.year;
                }
            }
        }

        Ok(Some(meta))
    }

    async fn resolve_tv(&self, mut meta: ParsedMetadata) -> Result<Option<ParsedMetadata>> {
        // Skip API only when every field — including show_year — is already present.
        if meta.tv_complete() && meta.episode_title.is_some() && meta.show_year.is_some() {
            return Ok(Some(meta));
        }

        if let (Some(tvdb), Some(show), Some(season_str), Some(episode_str)) = (
            &self.tvdb,
            meta.show.as_deref(),
            meta.season.as_deref(),
            meta.episode.as_deref(),
        ) {
            let season: u32 = season_str.parse().unwrap_or(0);
            let episode: u32 = episode_str.parse().unwrap_or(0);

            if let Ok(Some(api_meta)) = tvdb.search_episode(show, season, episode).await {
                if meta.episode_title.is_none() {
                    meta.episode_title = api_meta.episode_title;
                }
                if meta.show_year.is_none() {
                    meta.show_year = api_meta.show_year;
                }
                if let Some(canonical_show) = api_meta.show {
                    meta.show = Some(canonical_show);
                }
            }
        } else if let (Some(tmdb), Some(show), Some(season_str), Some(episode_str)) = (
            &self.tmdb,
            meta.show.as_deref(),
            meta.season.as_deref(),
            meta.episode.as_deref(),
        ) {
            let season: u32 = season_str.parse().unwrap_or(0);
            let episode: u32 = episode_str.parse().unwrap_or(0);

            if let Ok(Some(api_meta)) = tmdb.search_episode(show, season, episode).await {
                if meta.episode_title.is_none() {
                    meta.episode_title = api_meta.episode_title;
                }
                if meta.show_year.is_none() {
                    meta.show_year = api_meta.show_year;
                }
            }
        }

        Ok(Some(meta))
    }
}
