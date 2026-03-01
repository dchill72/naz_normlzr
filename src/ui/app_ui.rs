use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, AppState};
use crate::ui::{browser, path_input, preview, statusbar};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // ── Outer layout: title bar / body / status bar ───────────────────────────
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // body
            Constraint::Length(3), // status / keybindings
        ])
        .split(area);

    render_title_bar(f, app, outer[0]);
    render_body(f, app, outer[1]);
    statusbar::render(f, app, outer[2]);
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let dry_run_tag = if app.dry_run { " [DRY RUN]" } else { "" };
    let state_tag = match &app.state {
        AppState::Scanning => " | Scanning…",
        AppState::Browsing => " | Browsing",
        AppState::Previewing => " | Preview",
        AppState::Renaming { completed, total } => {
            let _ = (completed, total);
            " | Renaming…"
        }
        AppState::Done => " | Done",
        AppState::EditingPaths { .. } => " | Set Paths",
    };

    // For the Renaming state we need to own the string.
    let renaming_label;
    let state_label = if let AppState::Renaming { completed, total } = &app.state {
        renaming_label = format!(" | Renaming {completed}/{total}");
        renaming_label.as_str()
    } else {
        state_tag
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  nas_normlzr",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(dry_run_tag, Style::default().fg(Color::Yellow)),
        Span::styled(state_label, Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(title, area);
}

fn render_body(f: &mut Frame, app: &App, area: Rect) {
    match &app.state {
        AppState::Scanning => render_scanning(f, area),
        AppState::Done => render_done(f, app, area),
        AppState::EditingPaths { .. } => {
            // Render normal browser/preview in background, then overlay popup.
            render_split(f, app, area);
            path_input::render(f, app, area);
        }
        _ => render_split(f, app, area),
    }
}

fn render_scanning(f: &mut Frame, area: Rect) {
    let para = Paragraph::new("Scanning media directories…")
        .block(Block::default().borders(Borders::ALL).title("Scanning"))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(para, area);
}

fn render_done(f: &mut Frame, app: &App, area: Rect) {
    let para = Paragraph::new(app.status_msg.as_str())
        .block(Block::default().borders(Borders::ALL).title("Done"))
        .style(Style::default().fg(Color::Green));
    f.render_widget(para, area);
}

fn render_split(f: &mut Frame, app: &App, area: Rect) {
    // Split horizontally: browser (40 %) | preview (60 %)
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    browser::render(f, app, panes[0]);
    preview::render(f, app, panes[1]);
}
