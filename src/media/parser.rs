use anyhow::Result;
use regex::Regex;

use crate::config::{MoviePatterns, TvPatterns};
use crate::media::ParsedMetadata;

pub struct MovieParser {
    patterns: Vec<Regex>,
}

pub struct TvParser {
    patterns: Vec<Regex>,
}

impl MovieParser {
    pub fn new(config: &MoviePatterns) -> Result<Self> {
        let patterns = compile_patterns(&config.input_patterns)?;
        Ok(Self { patterns })
    }

    pub fn parse(&self, filename: &str) -> Option<ParsedMetadata> {
        let stem = strip_extension(filename);
        let normalized = normalize_separators(&stem);

        for pattern in &self.patterns {
            if let Some(caps) = pattern.captures(&normalized) {
                let title = caps
                    .name("title")
                    .map(|m| clean_name(m.as_str()));
                let year = caps
                    .name("year")
                    .map(|m| m.as_str().trim().to_string());

                return Some(ParsedMetadata {
                    title,
                    year,
                    ..Default::default()
                });
            }
        }
        None
    }
}

impl TvParser {
    pub fn new(config: &TvPatterns) -> Result<Self> {
        let patterns = compile_patterns(&config.input_patterns)?;
        Ok(Self { patterns })
    }

    pub fn parse(&self, filename: &str) -> Option<ParsedMetadata> {
        let stem = strip_extension(filename);
        let normalized = normalize_separators(&stem);

        for pattern in &self.patterns {
            if let Some(caps) = pattern.captures(&normalized) {
                let show = caps.name("show").map(|m| clean_name(m.as_str()));
                let season = caps
                    .name("season")
                    .map(|m| m.as_str().trim().to_string());
                let episode = caps
                    .name("episode")
                    .map(|m| m.as_str().trim().to_string());
                let episode_title = caps
                    .name("title")
                    .map(|m| clean_name(m.as_str()))
                    .filter(|s| !s.is_empty());

                return Some(ParsedMetadata {
                    show,
                    season,
                    episode,
                    episode_title,
                    ..Default::default()
                });
            }
        }
        None
    }
}

fn compile_patterns(patterns: &[String]) -> Result<Vec<Regex>> {
    patterns
        .iter()
        .map(|p| {
            Regex::new(p)
                .map_err(|e| anyhow::anyhow!("Invalid regex {:?}: {}", p, e))
        })
        .collect()
}

/// Remove the file extension (last `.xxx` segment).
fn strip_extension(filename: &str) -> String {
    match filename.rfind('.') {
        Some(pos) if pos > 0 => filename[..pos].to_string(),
        _ => filename.to_string(),
    }
}

/// Replace dots and underscores with spaces to normalize common filename styles.
/// E.g. "The.Matrix.1999.1080p" → "The Matrix 1999 1080p"
fn normalize_separators(s: &str) -> String {
    s.replace('.', " ").replace('_', " ")
}

/// Trim whitespace and trailing/leading separator characters.
fn clean_name(s: &str) -> String {
    s.trim()
        .trim_matches(|c: char| c == '-' || c == '.' || c == '_' || c == ' ')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MoviePatterns;

    fn default_movie_patterns() -> MoviePatterns {
        MoviePatterns {
            input_patterns: vec![
                r"(?i)(?P<title>.+?)\s*[\(\[]\s*(?P<year>\d{4})\s*[\)\]]".to_string(),
                r"(?i)(?P<title>.+?)\s+(?P<year>\d{4})(?:\s|$)".to_string(),
                r"(?P<title>.+)".to_string(),
            ],
            directory_template: String::new(),
            file_template: String::new(),
        }
    }

    #[test]
    fn parse_movie_with_parenthesis_year() {
        let parser = MovieParser::new(&default_movie_patterns()).unwrap();
        let meta = parser.parse("Alien (1979).mkv").unwrap();
        assert_eq!(meta.title.as_deref(), Some("Alien"));
        assert_eq!(meta.year.as_deref(), Some("1979"));
    }

    #[test]
    fn parse_movie_dot_separated() {
        let parser = MovieParser::new(&default_movie_patterns()).unwrap();
        let meta = parser.parse("The.Matrix.1999.1080p.BluRay.mkv").unwrap();
        assert_eq!(meta.title.as_deref(), Some("The Matrix"));
        assert_eq!(meta.year.as_deref(), Some("1999"));
    }

    #[test]
    fn parse_tv_sxxexx() {
        use crate::config::TvPatterns;
        let patterns = TvPatterns {
            input_patterns: vec![
                r"(?i)(?P<show>.+?)\s+[Ss](?P<season>\d{1,2})[Ee](?P<episode>\d{1,2})(?:\s+(?P<title>.+))?".to_string(),
            ],
            show_directory_template: String::new(),
            season_directory_template: String::new(),
            episode_file_template: String::new(),
            episode_file_template_no_title: String::new(),
        };
        let parser = TvParser::new(&patterns).unwrap();
        let meta = parser.parse("Breaking.Bad.S01E03.Pilot.mkv").unwrap();
        assert_eq!(meta.show.as_deref(), Some("Breaking Bad"));
        assert_eq!(meta.season.as_deref(), Some("01"));
        assert_eq!(meta.episode.as_deref(), Some("03"));
    }
}
