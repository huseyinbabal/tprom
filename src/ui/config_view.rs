use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.config_yaml.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No config loaded. Press <R> to refresh.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let search_lower = app.config_search_text.to_lowercase();
    let has_search = !app.config_search_text.is_empty();

    let lines: Vec<Line> = app
        .config_yaml
        .lines()
        .enumerate()
        .skip(app.config_scroll)
        .map(|(idx, line)| {
            let is_match = has_search && line.to_lowercase().contains(&search_lower);
            let is_current_match = has_search
                && !app.config_search_matches.is_empty()
                && app.config_search_matches.get(app.config_search_current) == Some(&idx);

            // Basic YAML syntax highlighting
            let style = if is_current_match {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_match {
                Style::default().bg(Color::Rgb(60, 60, 0)).fg(Color::White)
            } else if line.starts_with('#') {
                Style::default().fg(Color::DarkGray)
            } else if line.contains(':') {
                // Key part in cyan
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(Span::styled(line, style))
        })
        .collect();

    let title = if has_search {
        let match_info = if app.config_search_matches.is_empty() {
            "no matches".to_string()
        } else {
            format!(
                "{}/{}",
                app.config_search_current + 1,
                app.config_search_matches.len()
            )
        };
        format!(" Config [/{}  {}] ", app.config_search_text, match_info)
    } else {
        " Config ".to_string()
    };

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(paragraph, area);
}
