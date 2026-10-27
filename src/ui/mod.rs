pub mod alert_toast;
pub mod alerts_table;
pub mod config_view;
pub mod describe;
pub mod dialog;
pub mod exclusion_dialog;
pub mod exclusion_table;
pub mod graph_view;
pub mod header;
pub mod help;
pub mod info_view;
pub mod labels_view;
pub mod metrics_table;
pub mod multi_server_modal;
pub mod query_view;
pub mod resources;
pub mod rules_table;
pub mod server_dialog;
pub mod servers_table;
pub mod splash;
pub mod targets_table;
pub mod tsdb_view;

use crate::app::{App, Mode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Format a numeric value string from scientific notation to a human-readable number.
/// e.g. "1e+00" -> "1", "2.8e+01" -> "28", "2.77777777777778e+00" -> "2.7778"
pub fn format_alert_value(raw: &str) -> String {
    match raw.parse::<f64>() {
        Ok(v) => {
            if v == 0.0 {
                "0".to_string()
            } else if v.fract() == 0.0 && v.abs() < 1e15 {
                format!("{}", v as i64)
            } else {
                let s = format!("{:.4}", v);
                let s = s.trim_end_matches('0');
                let s = s.trim_end_matches('.');
                s.to_string()
            }
        }
        Err(_) => raw.to_string(),
    }
}

pub fn render(f: &mut Frame, app: &mut App) {
    if app.mode == Mode::Splash {
        splash::render(f, &app.splash_state);
        return;
    }

    if app.mode == Mode::ServerDialog {
        server_dialog::render(f, &app.server_dialog_state);
        return;
    }

    let show_search_bar = app.filter_active || app.config_search_active;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if show_search_bar {
            vec![
                Constraint::Length(6), // Header
                Constraint::Length(3), // Search bar
                Constraint::Min(1),    // Main content
                Constraint::Length(1), // Footer / status bar
            ]
        } else {
            vec![
                Constraint::Length(6), // Header
                Constraint::Length(0), // Search bar (hidden)
                Constraint::Min(1),    // Main content
                Constraint::Length(1), // Footer / status bar
            ]
        })
        .split(f.area());

    header::render(f, app, chunks[0]);

    // Render search bar at the top of content area
    if show_search_bar {
        let (prefix, text) = if app.filter_active {
            ("/", app.filter_text.as_str())
        } else {
            ("/", app.config_search_text.as_str())
        };

        let search_line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Yellow)),
            Span::styled(text, Style::default().fg(Color::White)),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::Rgb(255, 140, 0))
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]);
        let search_bar = Paragraph::new(search_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(255, 140, 0)))
                .title(Span::styled(
                    " Search ",
                    Style::default()
                        .fg(Color::Rgb(255, 140, 0))
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(search_bar, chunks[1]);
    }

    // Main content area
    let content_area = chunks[2];

    match app.mode {
        Mode::Describe => {
            describe::render(f, app, content_area);
        }
        _ => match app.active_resource.as_str() {
            "servers" => servers_table::render(f, app, content_area),
            "targets" => targets_table::render(f, app, content_area),
            "alerts" => alerts_table::render(f, app, content_area),
            "alertexclusion" => exclusion_table::render(f, app, content_area),
            "rules" => rules_table::render(f, app, content_area),
            "metrics" => metrics_table::render(f, app, content_area),
            "labels" => labels_view::render(f, app, content_area),
            "tsdb" => tsdb_view::render(f, app, content_area),
            "config" => config_view::render(f, app, content_area),
            "info" => info_view::render(f, app, content_area),
            "query" => {
                if app.graph_visible {
                    graph_view::render(f, app, content_area);
                } else {
                    query_view::render(f, app, content_area);
                }
            }
            _ => targets_table::render(f, app, content_area),
        },
    }

    // Loading spinner overlay in the content area
    if app.loading {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinner_chars[app.spinner_frame % spinner_chars.len()];
        let loading_text = Line::from(vec![
            Span::styled(
                format!("{} ", spinner),
                Style::default()
                    .fg(Color::Rgb(255, 140, 0))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Loading...", Style::default().fg(Color::DarkGray)),
        ]);
        // Center the spinner in the content area
        let popup_width = 20u16;
        let popup_height = 3u16;
        let popup_area = Rect {
            x: content_area.x + content_area.width.saturating_sub(popup_width) / 2,
            y: content_area.y + content_area.height.saturating_sub(popup_height) / 2,
            width: popup_width.min(content_area.width),
            height: popup_height.min(content_area.height),
        };
        f.render_widget(Clear, popup_area);
        let loading_widget = Paragraph::new(loading_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(255, 140, 0))),
            );
        f.render_widget(loading_widget, popup_area);
    }

    // Footer: status messages only
    if let Some(ref msg) = app.status_message {
        let color = if app.status_is_error {
            Color::Red
        } else {
            Color::DarkGray
        };
        let status = Paragraph::new(Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(color),
        )));
        f.render_widget(status, chunks[3]);
    }

    // New firing alerts stay on screen until dismissed
    alert_toast::render(f, app, content_area);

    // Render overlays
    if app.mode == Mode::Confirm {
        dialog::render(f, app);
    }

    if app.mode == Mode::Resources {
        resources::render(f, app);
    }

    if app.mode == Mode::MultiServerSelect {
        multi_server_modal::render(f, app);
    }

    if app.mode == Mode::ExclusionDialog {
        exclusion_dialog::render(f, &app.exclusion_dialog_state);
    }

    if app.mode == Mode::Help {
        help::render(f, app);
    }
}
