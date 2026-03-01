use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::media::ParsedMetadata;

const BASE_URL: &str = "https://api.themoviedb.org/3";

pub struct TmdbClient {
    api_key: String,
    client: Client,
}

impl TmdbClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    /// Search for a movie by title (and optional year).
    /// Returns the best match or `None` if no results.
    pub async fn search_movie(
        &self,
        title: &str,
        year: Option<&str>,
    ) -> Result<Option<ParsedMetadata>> {
        #[derive(Deserialize)]
        struct SearchResponse {
            results: Vec<MovieResult>,
        }
        #[derive(Deserialize)]
        struct MovieResult {
            title: String,
            release_date: Option<String>,
        }

        let mut req = self
            .client
            .get(format!("{BASE_URL}/search/movie"))
            .query(&[("api_key", self.api_key.as_str()), ("query", title)]);

        if let Some(y) = year {
            req = req.query(&[("year", y)]);
        }

        let resp: SearchResponse = req.send().await?.json().await?;

        let best = resp.results.into_iter().next();
        Ok(best.map(|r| ParsedMetadata {
            title: Some(r.title),
            year: r
                .release_date
                .as_deref()
                .and_then(|d| d.split('-').next())
                .map(|y| y.to_string()),
            ..Default::default()
        }))
    }

    /// Search for a TV episode via TMDB (as fallback when TVDB is unavailable).
    pub async fn search_episode(
        &self,
        show: &str,
        season: u32,
        episode: u32,
    ) -> Result<Option<ParsedMetadata>> {
        // Step 1: find series ID and premiere year.
        let (series_id, show_year) = match self.find_series_info(show).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        // Step 2: fetch episode details.
        #[derive(Deserialize)]
        struct EpisodeResult {
            name: String,
        }

        let resp: EpisodeResult = self
            .client
            .get(format!(
                "{BASE_URL}/tv/{series_id}/season/{season}/episode/{episode}"
            ))
            .query(&[("api_key", self.api_key.as_str())])
            .send()
            .await?
            .json()
            .await?;

        Ok(Some(ParsedMetadata {
            episode_title: Some(resp.name),
            show_year,
            ..Default::default()
        }))
    }

    /// Returns `(series_id, premiere_year)` for the best-matching TV series.
    async fn find_series_info(&self, show: &str) -> Result<Option<(u64, Option<String>)>> {
        #[derive(Deserialize)]
        struct SearchResponse {
            results: Vec<SeriesResult>,
        }
        #[derive(Deserialize)]
        struct SeriesResult {
            id: u64,
            first_air_date: Option<String>,
        }

        let resp: SearchResponse = self
            .client
            .get(format!("{BASE_URL}/search/tv"))
            .query(&[("api_key", self.api_key.as_str()), ("query", show)])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().next().map(|r| {
            let year = r
                .first_air_date
                .as_deref()
                .and_then(|d| d.split('-').next())
                .filter(|y| !y.is_empty())
                .map(|y| y.to_string());
            (r.id, year)
        }))
    }
}
