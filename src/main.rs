mod app;
mod config;
mod media;
mod metadata;
mod template;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "nas_normlzr",
    version,
    about = "Interactively normalize media paths on your NAS"
)]
struct Cli {
    /// Path to the TOML configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Preview changes without renaming any files
    #[arg(short = 'd', long)]
    dry_run: bool,

    /// Override the movies root directory from config
    #[arg(long, value_name = "PATH")]
    movies: Option<PathBuf>,

    /// Override the TV shows root directory from config
    #[arg(long, value_name = "PATH")]
    tv: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Log to file so we don't corrupt the TUI output.
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("nas_normlzr.log")
        .context("Failed to open nas_normlzr.log")?;

    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "nas_normlzr=info".into()),
        )
        .init();

    let mut config = config::Config::load(&cli.config).with_context(|| {
        format!(
            "Could not load config from '{}'. \
             Copy config.toml and edit the [roots] section.",
            cli.config.display()
        )
    })?;

    // CLI overrides take precedence over config file roots.
    if let Some(movies) = cli.movies {
        config.roots.movies = Some(movies);
    }
    if let Some(tv) = cli.tv {
        config.roots.tv_shows = Some(tv);
    }

    let mut application = app::App::new(config, cli.dry_run)?;
    application.run().await
}
