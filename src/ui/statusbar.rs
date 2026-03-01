use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, AppState};

/// Key hint: label + description.
struct Hint(&'static str, &'static str);

fn hints_for_state(state: &AppState) -> Vec<Hint> {
    match state {
        AppState::Scanning => vec![Hint("q", "quit")],
        AppState::Browsing => vec![
            Hint("Tab", "switch tab"),
            Hint("j/k", "navigate"),
            Hint("Enter", "preview"),
            Hint("a", "approve"),
            Hint("s", "skip"),
            Hint("A", "approve all"),
            Hint("S", "skip all"),
            Hint("R", "run renames"),
            Hint("r", "rescan"),
            Hint("p", "set paths"),
            Hint("q", "quit"),
        ],
        AppState::Previewing => vec![
            Hint("Tab", "switch tab"),
            Hint("j/k", "navigate"),
            Hint("Enter", "back"),
            Hint("a", "approve"),
            Hint("s", "skip"),
            Hint("A", "approve all"),
            Hint("R", "run renames"),
            Hint("p", "set paths"),
            Hint("q", "quit"),
        ],
        AppState::EditingPaths { .. } => vec![
            Hint("Tab", "switch field"),
            Hint("Enter", "apply & scan"),
            Hint("Esc", "cancel"),
        ],
        AppState::Renaming { .. } => vec![Hint("q", "quit")],
        AppState::Done => vec![Hint("q/Enter", "quit")],
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let hints = hints_for_state(&app.state);

    let mut spans: Vec<Span> = Vec::new();
    for (i, hint) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            format!("[{}]", hint.0),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", hint.1),
            Style::default().fg(Color::Gray),
        ));
    }

    // Second line: status message
    let status_line = Span::styled(
        app.status_msg.as_str(),
        Style::default().fg(Color::Yellow),
    );

    let content = vec![
        Line::from(spans),
        Line::from(vec![status_line]),
    ];

    let para = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(para, area);
}
