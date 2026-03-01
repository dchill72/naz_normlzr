use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, AppState, PathField};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let AppState::EditingPaths {
        movies,
        tv_shows,
        active,
    } = &app.state
    else {
        return;
    };

    let popup = centered_popup(64, 12, area);

    // Clear the background behind the popup.
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Set Media Roots ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Inner layout: padding / movies label+input / gap / tv label+input / hints
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top padding
            Constraint::Length(1), // "Movies:" label
            Constraint::Length(3), // movies input box
            Constraint::Length(1), // gap
            Constraint::Length(1), // "TV Shows:" label
            Constraint::Length(3), // tv input box
            Constraint::Min(0),    // hints
        ])
        .split(inner);

    // Movies field
    render_field(
        f,
        "Movies",
        movies,
        *active == PathField::Movies,
        rows[1],
        rows[2],
    );

    // TV Shows field
    render_field(
        f,
        "TV Shows",
        tv_shows,
        *active == PathField::TvShows,
        rows[4],
        rows[5],
    );

    // Hints row
    let hints = Paragraph::new(Line::from(vec![
        hint("[Tab]", "switch"),
        Span::raw("  "),
        hint("[Enter]", "apply & scan"),
        Span::raw("  "),
        hint("[Esc]", "cancel"),
    ]));
    f.render_widget(hints, rows[6]);
}

fn render_field(
    f: &mut Frame,
    label: &str,
    value: &str,
    is_active: bool,
    label_area: Rect,
    input_area: Rect,
) {
    let label_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    f.render_widget(Paragraph::new(Span::styled(label, label_style)), label_area);

    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };
    let display_value = if is_active {
        // Append a blinking cursor character.
        format!("{value}_")
    } else {
        value.to_string()
    };

    let input = Paragraph::new(display_value).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(input, input_area);
}

fn hint(key: &'static str, desc: &'static str) -> Span<'static> {
    // Return key span; desc is appended separately so we can style them.
    // Build as a single span for simplicity.
    Span::styled(
        format!("{key} {desc}"),
        Style::default().fg(Color::DarkGray),
    )
}

/// Return a centered `Rect` of the requested size within `area`.
fn centered_popup(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
