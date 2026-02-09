pub mod alerts_table;
pub mod config_view;
pub mod describe;
pub mod dialog;
pub mod header;
pub mod info_view;
pub mod labels_view;
pub mod metrics_table;
pub mod resources;
pub mod rules_table;
pub mod server_dialog;
pub mod servers_table;
pub mod splash;
pub mod targets_table;
pub mod tsdb_view;

use crate::app::{App, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if app.mode == Mode::Splash {
        splash::render(f, &app.splash_state);
        return;
    }

    if app.mode == Mode::ServerDialog {
        server_dialog::render(f, &app.server_dialog_state);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header
            Constraint::Min(1),   // Main content
            Constraint::Length(1), // Footer / status bar
        ])
        .split(f.area());

    header::render(f, app, chunks[0]);

    match app.mode {
        Mode::Describe => {
            describe::render(f, app, chunks[1]);
        }
        _ => match app.active_resource.as_str() {
            "servers" => servers_table::render(f, app, chunks[1]),
            "targets" => targets_table::render(f, app, chunks[1]),
            "alerts" => alerts_table::render(f, app, chunks[1]),
            "rules" => rules_table::render(f, app, chunks[1]),
            "metrics" => metrics_table::render(f, app, chunks[1]),
            "labels" => labels_view::render(f, app, chunks[1]),
            "tsdb" => tsdb_view::render(f, app, chunks[1]),
            "config" => config_view::render(f, app, chunks[1]),
            "info" => info_view::render(f, app, chunks[1]),
            _ => targets_table::render(f, app, chunks[1]),
        },
    }

    // Render filter bar in footer if active
    if app.filter_active {
        let filter_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::styled(&app.filter_text, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ]);
        let filter_bar = Paragraph::new(filter_line);
        f.render_widget(filter_bar, chunks[2]);
    } else if app.config_search_active {
        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::styled(&app.config_search_text, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ]);
        let search_bar = Paragraph::new(search_line);
        f.render_widget(search_bar, chunks[2]);
    } else if let Some(ref msg) = app.status_message {
        let status = Paragraph::new(Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(status, chunks[2]);
    }

    // Render overlays
    if app.mode == Mode::Confirm {
        dialog::render(f, app);
    }

    if app.mode == Mode::Resources {
        resources::render(f, app);
    }
}
