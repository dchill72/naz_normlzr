use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::App;
use crate::media::RenameStatus;

/// Status badge shown before the filename.
fn status_badge(status: &RenameStatus) -> (&'static str, Color) {
    match status {
        RenameStatus::Pending => ("[ ]", Color::Yellow),
        RenameStatus::Approved => ("[✓]", Color::Green),
        RenameStatus::Skipped => ("[-]", Color::DarkGray),
        RenameStatus::Done => ("[✓]", Color::Cyan),
        RenameStatus::AlreadyCorrect => ("[=]", Color::Blue),
        RenameStatus::LoadingMetadata => ("[…]", Color::Magenta),
        RenameStatus::Error(_) => ("[!]", Color::Red),
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pending = app.pending_count();
    let approved = app.approved_count();
    let total = app.tab_file_count();
    let title = format!(
        " {} ({} total, {} pending, {} approved) ",
        app.active_tab.label(),
        total,
        pending,
        approved
    );

    let visible = app.visible_file_indices();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&idx| {
            let file = &app.files[idx];
            let (badge, badge_color) = status_badge(&file.status);
            let name = file.display_name();
            // Truncate name to fit the panel width (rough estimate).
            let max_name_len = area.width.saturating_sub(8) as usize;
            let name = if name.chars().count() > max_name_len {
                let prefix: String = name
                    .chars()
                    .take(max_name_len.saturating_sub(1))
                    .collect();
                format!("{prefix}…")
            } else {
                name.to_string()
            };

            ListItem::new(Line::from(vec![
                Span::styled(badge, Style::default().fg(badge_color)),
                Span::raw(" "),
                Span::raw(name),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.selected_idx));
    }

    f.render_stateful_widget(list, area, &mut state);
}
