use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::OnceCell;

use crate::media::ParsedMetadata;

const BASE_URL: &str = "https://api4.thetvdb.com/v4";

pub struct TvdbClient {
    api_key: String,
    client: Client,
    token: OnceCell<String>,
}

impl TvdbClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            token: OnceCell::new(),
        }
    }

    /// Authenticate and cache the bearer token.
    async fn bearer_token(&self) -> Result<&str> {
        self.token
            .get_or_try_init(|| async {
                #[derive(serde::Serialize)]
                struct LoginRequest<'a> {
                    apikey: &'a str,
                }
                #[derive(Deserialize)]
                struct LoginResponse {
                    data: TokenData,
                }
                #[derive(Deserialize)]
                struct TokenData {
                    token: String,
                }

                let resp: LoginResponse = self
                    .client
                    .post(format!("{BASE_URL}/login"))
                    .json(&LoginRequest {
                        apikey: &self.api_key,
                    })
                    .send()
                    .await?
                    .json()
                    .await?;

                Ok::<String, anyhow::Error>(resp.data.token)
            })
            .await
            .map(|s| s.as_str())
    }

    /// Search for an episode by show name, season, and episode number.
    pub async fn search_episode(
        &self,
        show: &str,
        season: u32,
        episode: u32,
    ) -> Result<Option<ParsedMetadata>> {
        let token = self.bearer_token().await?;

        // Step 1: find series ID and premiere year.
        let (series_id, show_year) = match self.find_series_info(token, show).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        // Step 2: fetch episode.
        #[derive(Deserialize)]
        struct EpisodesResponse {
            data: EpisodesData,
        }
        #[derive(Deserialize)]
        struct EpisodesData {
            episodes: Vec<EpisodeResult>,
        }
        #[derive(Deserialize)]
        struct EpisodeResult {
            name: Option<String>,
        }

        let resp: EpisodesResponse = self
            .client
            .get(format!("{BASE_URL}/series/{series_id}/episodes/default"))
            .bearer_auth(token)
            .query(&[
                ("season", season.to_string()),
                ("episodeNumber", episode.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;

        let ep_title = resp
            .data
            .episodes
            .into_iter()
            .next()
            .and_then(|e| e.name)
            .filter(|n| !n.is_empty());

        Ok(Some(ParsedMetadata {
            episode_title: ep_title,
            show_year,
            ..Default::default()
        }))
    }

    /// Returns `(series_id, premiere_year)` for the best-matching series.
    async fn find_series_info(
        &self,
        token: &str,
        show: &str,
    ) -> Result<Option<(u64, Option<String>)>> {
        #[derive(Deserialize)]
        struct SearchResponse {
            data: Vec<SeriesResult>,
        }
        #[derive(Deserialize)]
        struct SeriesResult {
            id: u64,
            /// TVDB v4 search returns the premiere year as a string.
            year: Option<String>,
        }

        let resp: SearchResponse = self
            .client
            .get(format!("{BASE_URL}/search"))
            .bearer_auth(token)
            .query(&[("query", show), ("type", "series")])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp
            .data
            .into_iter()
            .next()
            .map(|r| (r.id, r.year.filter(|y| !y.is_empty()))))
    }
}
