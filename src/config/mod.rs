use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub roots: Roots,
    pub patterns: Patterns,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Roots {
    pub movies: Option<PathBuf>,
    pub tv_shows: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Patterns {
    pub movies: MoviePatterns,
    pub tv_shows: TvPatterns,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoviePatterns {
    pub input_patterns: Vec<String>,
    pub directory_template: String,
    pub file_template: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TvPatterns {
    pub input_patterns: Vec<String>,
    pub show_directory_template: String,
    pub season_directory_template: String,
    pub episode_file_template: String,
    pub episode_file_template_no_title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    #[serde(default)]
    pub tmdb_api_key: String,
    #[serde(default)]
    pub tvdb_api_key: String,
    #[serde(default = "default_true")]
    pub enable_api_fallback: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            tmdb_api_key: String::new(),
            tvdb_api_key: String::new(),
            enable_api_fallback: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiConfig {
    #[serde(default = "default_browser_height")]
    pub browser_height: u16,
}

fn default_browser_height() -> u16 {
    20
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            browser_height: default_browser_height(),
        }
    }
}

impl Config {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        Ok(config)
    }

    /// Returns true if at least one API key is set and fallback is enabled.
    pub fn api_enabled(&self) -> bool {
        self.api.enable_api_fallback
            && (!self.api.tmdb_api_key.is_empty() || !self.api.tvdb_api_key.is_empty())
    }
}
