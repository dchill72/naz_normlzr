pub mod app_ui;
pub mod browser;
pub mod path_input;
pub mod preview;
pub mod statusbar;

use ratatui::Frame;

use crate::app::App;

/// Top-level render entry point called every frame.
pub fn render(f: &mut Frame, app: &App) {
    app_ui::render(f, app);
}
