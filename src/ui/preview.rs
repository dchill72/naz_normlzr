use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::media::RenameStatus;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ")
        .border_style(Style::default().fg(Color::DarkGray));

    if app.files.is_empty() {
        let para = Paragraph::new("No files found.\nCheck your config.toml roots.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(para, area);
        return;
    }

    let file = &app.files[app.selected_idx];
    let inner = block.inner(area);
    f.render_widget(block, area);

    // ── Layout: current / proposed / metadata ────────────────────────────────
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // current path
            Constraint::Length(3),  // proposed path
            Constraint::Min(4),     // metadata / status info
        ])
        .split(inner);

    // Current path
    render_path_section(
        f,
        " Current ",
        &file.path.display().to_string(),
        Color::Gray,
        sections[0],
    );

    // Proposed path
    match &file.proposed_path {
        Some(proposed) => {
            let color = if file.status == RenameStatus::AlreadyCorrect {
                Color::Blue
            } else {
                Color::Green
            };
            render_path_section(
                f,
                " Proposed ",
                &proposed.display().to_string(),
                color,
                sections[1],
            );
        }
        None => {
            let msg = match &file.status {
                RenameStatus::LoadingMetadata => "Loading metadata from API…".to_string(),
                RenameStatus::Error(e) => format!("Error: {e}"),
                RenameStatus::AlreadyCorrect => "Path is already correct.".to_string(),
                _ => "No proposed path — check config or parsing.".to_string(),
            };
            render_path_section(f, " Proposed ", &msg, Color::DarkGray, sections[1]);
        }
    }

    // Metadata panel
    let meta_lines = build_metadata_lines(file);
    let meta_para = Paragraph::new(meta_lines)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Metadata "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(meta_para, sections[2]);
}

fn render_path_section(f: &mut Frame, title: &str, path: &str, color: Color, area: Rect) {
    let para = Paragraph::new(path)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(title, Style::default().fg(color))),
        )
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn build_metadata_lines(file: &crate::media::MediaFile) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let source = if file.resolved_metadata.is_some() {
        "API"
    } else if file.parsed_metadata.is_some() {
        "filename"
    } else {
        "none"
    };

    lines.push(Line::from(vec![
        Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
        Span::styled(source, Style::default().fg(Color::White)),
    ]));

    if let Some(meta) = file.effective_metadata() {
        let fields: Vec<(&str, &Option<String>)> = match file.media_type {
            crate::media::MediaType::Movie => vec![
                ("title", &meta.title),
                ("year", &meta.year),
            ],
            crate::media::MediaType::TvEpisode => vec![
                ("show", &meta.show),
                ("show year", &meta.show_year),
                ("season", &meta.season),
                ("episode", &meta.episode),
                ("episode title", &meta.episode_title),
            ],
        };

        for (label, value) in fields {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{label}: "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    value.clone().unwrap_or_else(|| "—".into()),
                    Style::default()
                        .fg(if value.is_some() {
                            Color::White
                        } else {
                            Color::Red
                        })
                        .add_modifier(if value.is_none() {
                            Modifier::empty()
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("status: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            file.status.label().to_string(),
            Style::default().fg(status_color(&file.status)),
        ),
    ]));

    lines
}

fn status_color(status: &crate::media::RenameStatus) -> Color {
    match status {
        RenameStatus::Pending => Color::Yellow,
        RenameStatus::Approved => Color::Green,
        RenameStatus::Skipped => Color::DarkGray,
        RenameStatus::Done => Color::Cyan,
        RenameStatus::AlreadyCorrect => Color::Blue,
        RenameStatus::LoadingMetadata => Color::Magenta,
        RenameStatus::Error(_) => Color::Red,
    }
}
