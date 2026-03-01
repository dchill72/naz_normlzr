use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::media::{MediaFile, MediaType};
use crate::template::{render, Vars};

/// Compute the desired absolute path for a `MediaFile` based on config templates.
///
/// Returns `Ok(None)` when metadata is incomplete (API lookup needed).
pub fn compute_proposed_path(file: &MediaFile, config: &Config) -> Result<Option<PathBuf>> {
    let meta = match file.effective_metadata() {
        Some(m) => m,
        None => return Ok(None),
    };

    let ext = file
        .path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    match file.media_type {
        MediaType::Movie => {
            let root = match config.roots.movies.as_ref() {
                Some(r) => r,
                None => return Ok(None),
            };
            if meta.title.is_none() || meta.year.is_none() {
                return Ok(None);
            }
            let mut vars = Vars::new();
            vars.insert("title".into(), meta.title.clone().unwrap_or_default());
            vars.insert("year".into(), meta.year.clone().unwrap_or_default());
            vars.insert("ext".into(), ext);

            let dir_name = render(&config.patterns.movies.directory_template, &vars)
                .context("Failed to render movie directory_template")?;
            let file_name = render(&config.patterns.movies.file_template, &vars)
                .context("Failed to render movie file_template")?;

            Ok(Some(root.join(dir_name).join(file_name)))
        }

        MediaType::TvEpisode => {
            let root = match config.roots.tv_shows.as_ref() {
                Some(r) => r,
                None => return Ok(None),
            };
            if meta.show.is_none() || meta.season.is_none() || meta.episode.is_none() {
                return Ok(None);
            }
            let mut vars = Vars::new();
            vars.insert("show".into(), meta.show.clone().unwrap_or_default());
            vars.insert(
                "show_year".into(),
                meta.show_year.clone().unwrap_or_default(),
            );
            vars.insert("season".into(), meta.season.clone().unwrap_or_default());
            vars.insert("episode".into(), meta.episode.clone().unwrap_or_default());
            vars.insert(
                "title".into(),
                meta.episode_title.clone().unwrap_or_default(),
            );
            vars.insert("ext".into(), ext);

            let show_dir = clean_empty_parens(
                render(&config.patterns.tv_shows.show_directory_template, &vars)
                    .context("Failed to render show_directory_template")?,
            );
            let season_dir =
                render(&config.patterns.tv_shows.season_directory_template, &vars)
                    .context("Failed to render season_directory_template")?;

            let file_name = clean_empty_parens(if meta.episode_title.is_some() {
                render(&config.patterns.tv_shows.episode_file_template, &vars)
                    .context("Failed to render episode_file_template")?
            } else {
                render(
                    &config.patterns.tv_shows.episode_file_template_no_title,
                    &vars,
                )
                .context("Failed to render episode_file_template_no_title")?
            });

            Ok(Some(root.join(show_dir).join(season_dir).join(file_name)))
        }
    }
}

/// Rename (move) a file from `old_path` to `new_path`.
///
/// Creates parent directories as needed.
/// If `dry_run` is true the filesystem is not modified.
pub fn execute_rename(old_path: &Path, new_path: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        tracing::info!(
            "[dry-run] would rename: {} → {}",
            old_path.display(),
            new_path.display()
        );
        return Ok(());
    }

    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create directory: {}", parent.display())
        })?;
    }

    std::fs::rename(old_path, new_path).with_context(|| {
        format!(
            "Failed to rename {} → {}",
            old_path.display(),
            new_path.display()
        )
    })?;

    // Remove old parent directory if it is now empty.
    if let Some(old_parent) = old_path.parent() {
        let _ = try_remove_empty_dir(old_parent);
    }

    tracing::info!(
        "Renamed: {} → {}",
        old_path.display(),
        new_path.display()
    );
    Ok(())
}

/// Remove empty-parenthesis artifacts left when a template variable is absent.
/// e.g. `"Show Name ()"` → `"Show Name"` when `show_year` is not yet known.
fn clean_empty_parens(s: String) -> String {
    s.replace(" ()", "").replace("()", "").trim().to_string()
}

/// Remove a directory only if it is empty; silently ignore errors.
fn try_remove_empty_dir(dir: &Path) {
    if dir.is_dir() {
        let is_empty = std::fs::read_dir(dir)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(dir);
        }
    }
}
