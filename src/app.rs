use anyhow::Result;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::media::renamer::{compute_proposed_path, execute_rename};
use crate::media::scanner;
use crate::media::{MediaFile, ParsedMetadata, RenameStatus};
use crate::metadata::MetadataResolver;
use crate::ui;

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathField {
    Movies,
    TvShows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Scanning,
    Browsing,
    Previewing,
    Renaming { completed: usize, total: usize },
    Done,
    /// Path editor popup — edits both roots before rescanning.
    EditingPaths {
        movies: String,
        tv_shows: String,
        active: PathField,
    },
}

// ── Internal messages (background task → main loop) ──────────────────────────

pub enum AppMsg {
    ScanComplete(Vec<MediaFile>),
    ScanError(String),
    MetadataResolved {
        idx: usize,
        metadata: ParsedMetadata,
        proposed_path: PathBuf,
    },
    MetadataFailed {
        idx: usize,
        error: String,
    },
    RenameComplete {
        idx: usize,
    },
    RenameFailed {
        idx: usize,
        error: String,
    },
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub state: AppState,
    pub config: Config,
    pub files: Vec<MediaFile>,
    pub selected_idx: usize,
    pub scroll_offset: usize,
    pub status_msg: String,
    pub dry_run: bool,
    msg_tx: mpsc::Sender<AppMsg>,
    msg_rx: mpsc::Receiver<AppMsg>,
}

impl App {
    pub fn new(config: Config, dry_run: bool) -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::channel(256);
        Ok(Self {
            state: AppState::Scanning,
            config,
            files: Vec::new(),
            selected_idx: 0,
            scroll_offset: 0,
            status_msg: "Scanning…".into(),
            dry_run,
            msg_tx,
            msg_rx,
        })
    }

    // ── Main loop ─────────────────────────────────────────────────────────────

    pub async fn run(&mut self) -> Result<()> {
        let mut stdout = std::io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen)?;

        // Restore terminal on panic.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            hook(info);
        }));

        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        self.start_scan();

        let mut events = EventStream::new();

        loop {
            terminal.draw(|f| ui::render(f, self))?;

            tokio::select! {
                Some(Ok(event)) = events.next() => {
                    if self.handle_terminal_event(event).await? {
                        break;
                    }
                }
                Some(msg) = self.msg_rx.recv() => {
                    self.handle_msg(msg).await?;
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    // ── Event handling ────────────────────────────────────────────────────────

    async fn handle_terminal_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) => self.handle_key(key).await,
            Event::Resize(_, _) => Ok(false),
            _ => Ok(false),
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // Ctrl-C always quits.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(true);
        }

        match self.state.clone() {
            AppState::Scanning => {
                if key.code == KeyCode::Char('q') {
                    return Ok(true);
                }
            }

            AppState::Browsing | AppState::Previewing => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                    KeyCode::Down | KeyCode::Char('j') => self.select_next(),
                    KeyCode::Up | KeyCode::Char('k') => self.select_prev(),
                    KeyCode::Enter => {
                        self.state = match self.state {
                            AppState::Browsing => AppState::Previewing,
                            _ => AppState::Browsing,
                        };
                    }
                    KeyCode::Char('a') => self.approve_selected(),
                    KeyCode::Char('s') => self.skip_selected(),
                    KeyCode::Char('A') => self.approve_all(),
                    KeyCode::Char('S') => self.skip_all(),
                    KeyCode::Char('R') => self.start_rename_approved().await?,
                    KeyCode::Char('r') => {
                        self.state = AppState::Scanning;
                        self.files.clear();
                        self.selected_idx = 0;
                        self.scroll_offset = 0;
                        self.status_msg = "Scanning…".into();
                        self.start_scan();
                    }
                    KeyCode::Char('p') => {
                        let movies = self
                            .config
                            .roots
                            .movies
                            .as_deref()
                            .and_then(|p| p.to_str())
                            .unwrap_or("")
                            .to_string();
                        let tv_shows = self
                            .config
                            .roots
                            .tv_shows
                            .as_deref()
                            .and_then(|p| p.to_str())
                            .unwrap_or("")
                            .to_string();
                        self.state = AppState::EditingPaths {
                            movies,
                            tv_shows,
                            active: PathField::Movies,
                        };
                    }
                    _ => {}
                }
            }

            AppState::EditingPaths {
                mut movies,
                mut tv_shows,
                active,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        self.state = AppState::Browsing;
                    }
                    KeyCode::Tab => {
                        let next = if active == PathField::Movies {
                            PathField::TvShows
                        } else {
                            PathField::Movies
                        };
                        self.state = AppState::EditingPaths {
                            movies,
                            tv_shows,
                            active: next,
                        };
                    }
                    KeyCode::Enter => {
                        self.config.roots.movies = if movies.trim().is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(movies.trim()))
                        };
                        self.config.roots.tv_shows = if tv_shows.trim().is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(tv_shows.trim()))
                        };
                        self.state = AppState::Scanning;
                        self.files.clear();
                        self.selected_idx = 0;
                        self.scroll_offset = 0;
                        self.status_msg = "Scanning…".into();
                        self.start_scan();
                    }
                    KeyCode::Backspace => {
                        match active {
                            PathField::Movies => {
                                movies.pop();
                            }
                            PathField::TvShows => {
                                tv_shows.pop();
                            }
                        }
                        self.state = AppState::EditingPaths {
                            movies,
                            tv_shows,
                            active,
                        };
                    }
                    KeyCode::Char(c) => {
                        match active {
                            PathField::Movies => movies.push(c),
                            PathField::TvShows => tv_shows.push(c),
                        }
                        self.state = AppState::EditingPaths {
                            movies,
                            tv_shows,
                            active,
                        };
                    }
                    _ => {}
                }
            }

            AppState::Renaming { .. } => {
                if key.code == KeyCode::Char('q') {
                    return Ok(true);
                }
            }

            AppState::Done => {
                if matches!(
                    key.code,
                    KeyCode::Char('q') | KeyCode::Enter | KeyCode::Esc
                ) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    // ── Background message handling ───────────────────────────────────────────

    async fn handle_msg(&mut self, msg: AppMsg) -> Result<()> {
        match msg {
            AppMsg::ScanComplete(files) => {
                let count = files.len();
                self.files = files;
                self.state = AppState::Browsing;
                self.status_msg = format!("Found {count} files. [a] approve  [s] skip  [R] run renames");
                // Kick off API lookups for files that need them.
                self.start_metadata_lookups();
            }

            AppMsg::ScanError(e) => {
                self.state = AppState::Browsing;
                self.status_msg = format!("Scan error: {e}");
            }

            AppMsg::MetadataResolved {
                idx,
                metadata,
                proposed_path,
            } => {
                if let Some(file) = self.files.get_mut(idx) {
                    file.resolved_metadata = Some(metadata);
                    file.proposed_path = Some(proposed_path);
                    file.status = if file.needs_rename() {
                        RenameStatus::Pending
                    } else {
                        RenameStatus::AlreadyCorrect
                    };
                }
            }

            AppMsg::MetadataFailed { idx, error } => {
                if let Some(file) = self.files.get_mut(idx) {
                    file.status = RenameStatus::Error(error);
                }
            }

            AppMsg::RenameComplete { idx } => {
                if let Some(file) = self.files.get_mut(idx) {
                    file.status = RenameStatus::Done;
                }
                self.advance_rename_progress();
            }

            AppMsg::RenameFailed { idx, error } => {
                if let Some(file) = self.files.get_mut(idx) {
                    file.status = RenameStatus::Error(error);
                }
                self.advance_rename_progress();
            }
        }
        Ok(())
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn select_next(&mut self) {
        if !self.files.is_empty() {
            self.selected_idx = (self.selected_idx + 1).min(self.files.len() - 1);
        }
    }

    fn select_prev(&mut self) {
        self.selected_idx = self.selected_idx.saturating_sub(1);
    }

    // ── Actions ───────────────────────────────────────────────────────────────

    fn approve_selected(&mut self) {
        if let Some(file) = self.files.get_mut(self.selected_idx) {
            if file.status == RenameStatus::Pending || file.status == RenameStatus::Skipped {
                file.status = RenameStatus::Approved;
            }
        }
    }

    fn skip_selected(&mut self) {
        if let Some(file) = self.files.get_mut(self.selected_idx) {
            if file.status == RenameStatus::Pending || file.status == RenameStatus::Approved {
                file.status = RenameStatus::Skipped;
            }
        }
    }

    fn approve_all(&mut self) {
        for file in &mut self.files {
            if file.status == RenameStatus::Pending {
                file.status = RenameStatus::Approved;
            }
        }
        self.status_msg = "All pending renames approved. Press [R] to execute.".into();
    }

    fn skip_all(&mut self) {
        for file in &mut self.files {
            if file.status == RenameStatus::Pending {
                file.status = RenameStatus::Skipped;
            }
        }
        self.status_msg = "All pending renames skipped.".into();
    }

    // ── Background tasks ──────────────────────────────────────────────────────

    fn start_scan(&self) {
        let tx = self.msg_tx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            match scanner::scan_all(&config) {
                Ok(files) => {
                    let _ = tx.send(AppMsg::ScanComplete(files)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppMsg::ScanError(e.to_string())).await;
                }
            }
        });
    }

    fn start_metadata_lookups(&self) {
        if !self.config.api_enabled() {
            return;
        }
        let resolver = MetadataResolver::new(&self.config);
        let tx = self.msg_tx.clone();
        let config = self.config.clone();

        // Collect indices that need API lookup.
        let indices: Vec<usize> = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.status == RenameStatus::LoadingMetadata)
            .map(|(i, _)| i)
            .collect();

        // Clone files for background task.
        let files: Vec<MediaFile> = indices.iter().map(|&i| self.files[i].clone()).collect();

        tokio::spawn(async move {
            for (file, idx) in files.into_iter().zip(indices) {
                match resolver.resolve(&file).await {
                    Ok(Some(meta)) => {
                        // Recompute proposed path with resolved metadata.
                        let mut enriched = file.clone();
                        enriched.resolved_metadata = Some(meta.clone());
                        match compute_proposed_path(&enriched, &config) {
                            Ok(Some(proposed)) => {
                                let _ = tx
                                    .send(AppMsg::MetadataResolved {
                                        idx,
                                        metadata: meta,
                                        proposed_path: proposed,
                                    })
                                    .await;
                            }
                            _ => {
                                let _ = tx
                                    .send(AppMsg::MetadataFailed {
                                        idx,
                                        error: "Could not compute path after API lookup".into(),
                                    })
                                    .await;
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = tx
                            .send(AppMsg::MetadataFailed {
                                idx,
                                error: "No metadata found".into(),
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(AppMsg::MetadataFailed {
                                idx,
                                error: e.to_string(),
                            })
                            .await;
                    }
                }
            }
        });
    }

    async fn start_rename_approved(&mut self) -> Result<()> {
        let approved: Vec<usize> = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.status == RenameStatus::Approved)
            .map(|(i, _)| i)
            .collect();

        if approved.is_empty() {
            self.status_msg = "No approved renames. Press [a] to approve files first.".into();
            return Ok(());
        }

        let total = approved.len();
        self.state = AppState::Renaming {
            completed: 0,
            total,
        };
        self.status_msg = format!("Renaming 0 / {total}…");

        let tx = self.msg_tx.clone();
        let dry_run = self.dry_run;

        let pairs: Vec<(usize, PathBuf, PathBuf)> = approved
            .iter()
            .filter_map(|&i| {
                let f = &self.files[i];
                f.proposed_path
                    .as_ref()
                    .map(|p| (i, f.path.clone(), p.clone()))
            })
            .collect();

        tokio::spawn(async move {
            for (idx, old_path, new_path) in pairs {
                match execute_rename(&old_path, &new_path, dry_run) {
                    Ok(()) => {
                        let _ = tx.send(AppMsg::RenameComplete { idx }).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(AppMsg::RenameFailed {
                                idx,
                                error: e.to_string(),
                            })
                            .await;
                    }
                }
            }
        });

        Ok(())
    }

    fn advance_rename_progress(&mut self) {
        if let AppState::Renaming { completed, total } = &mut self.state {
            *completed += 1;
            let done = *completed;
            let tot = *total;
            self.status_msg = format!("Renaming {done} / {tot}…");
            if done >= tot {
                self.state = AppState::Done;
                let errors = self
                    .files
                    .iter()
                    .filter(|f| matches!(f.status, RenameStatus::Error(_)))
                    .count();
                let renamed = self
                    .files
                    .iter()
                    .filter(|f| f.status == RenameStatus::Done)
                    .count();
                self.status_msg =
                    format!("Done! {renamed} renamed, {errors} errors. Press [q] to quit.");
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    pub fn approved_count(&self) -> usize {
        self.files
            .iter()
            .filter(|f| f.status == RenameStatus::Approved)
            .count()
    }

    pub fn pending_count(&self) -> usize {
        self.files
            .iter()
            .filter(|f| f.status == RenameStatus::Pending)
            .count()
    }
}
