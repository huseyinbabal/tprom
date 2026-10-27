use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 80, f.area());
    f.render_widget(Clear, area);

    let mut lines: Vec<Line<'static>> = vec![Line::from("")];

    // Context-specific section first
    match app.active_resource.as_str() {
        "servers" => {
            lines.push(section("Servers"));
            lines.push(key_line("c", "Connect to server"));
            lines.push(key_line("a", "Add new server"));
            lines.push(key_line("D", "Delete server"));
            lines.push(Line::from(""));
        }
        "alerts" => {
            lines.push(section("Alerts"));
            lines.push(key_line("d / Enter", "Describe alert"));
            lines.push(key_line("/", "Filter alerts"));
            lines.push(key_line("x", "Add exclusion rule"));
            lines.push(key_line("R", "Refresh"));
            lines.push(key_line("m", "Multi-server mode"));
            lines.push(key_line(
                "a",
                "Toggle auto-refresh (needed for alert sound)",
            ));
            lines.push(key_line("+ / -", "Auto-refresh interval"));
            lines.push(key_line("b", "Toggle sound on new critical alert"));
            lines.push(key_line("Esc", "Dismiss alert notifications"));
            lines.push(Line::from(""));
            if app.multi_server_mode_active {
                lines.push(section("Multi-Server"));
                lines.push(key_line("h / Left", "Collapse group"));
                lines.push(key_line("l / Right", "Expand group"));
                lines.push(key_line("W", "Collapse all"));
                lines.push(key_line("E", "Expand all"));
                lines.push(Line::from(""));
            }
        }
        "alertexclusion" => {
            lines.push(section("Alert Exclusions"));
            lines.push(key_line("x", "Add exclusion"));
            lines.push(key_line("d / Enter", "Edit exclusion"));
            lines.push(key_line("D", "Delete exclusion"));
            lines.push(Line::from(""));
        }
        "targets" => {
            lines.push(section("Targets"));
            lines.push(key_line("d / Enter", "Describe target"));
            lines.push(key_line("/", "Filter targets"));
            lines.push(key_line("R", "Refresh"));
            lines.push(key_line("a", "Toggle auto-refresh"));
            lines.push(Line::from(""));
        }
        "rules" => {
            lines.push(section("Rules"));
            lines.push(key_line("d / Enter", "Describe rule"));
            lines.push(key_line("/", "Filter rules"));
            lines.push(key_line("R", "Refresh"));
            lines.push(key_line("a", "Toggle auto-refresh"));
            lines.push(Line::from(""));
        }
        "metrics" => {
            lines.push(section("Metrics"));
            lines.push(key_line("d / Enter", "Describe metric"));
            lines.push(key_line("/", "Filter metrics"));
            lines.push(key_line("R", "Refresh"));
            lines.push(key_line("a", "Toggle auto-refresh"));
            lines.push(Line::from(""));
        }
        "labels" => {
            lines.push(section("Labels"));
            lines.push(key_line("Tab", "Switch panel"));
            lines.push(key_line("R", "Refresh"));
            lines.push(key_line("a", "Toggle auto-refresh"));
            lines.push(Line::from(""));
        }
        "query" => {
            lines.push(section("Query"));
            lines.push(key_line("Tab", "Switch input / results"));
            lines.push(key_line("Enter", "Execute query"));
            lines.push(key_line("v", "Graph visualization"));
            lines.push(key_line("m", "Multi-server mode"));
            lines.push(key_line("Up / Down", "Query history (in input)"));
            lines.push(Line::from(""));
            if app.multi_server_mode_active {
                lines.push(section("Multi-Server"));
                lines.push(key_line("h / Left", "Collapse group"));
                lines.push(key_line("l / Right", "Expand group"));
                lines.push(key_line("W", "Collapse all"));
                lines.push(key_line("E", "Expand all"));
                lines.push(Line::from(""));
            }
        }
        "config" => {
            lines.push(section("Config"));
            lines.push(key_line("/", "Search"));
            lines.push(key_line("n / N", "Next / prev match"));
            lines.push(key_line("R", "Refresh"));
            lines.push(Line::from(""));
        }
        "tsdb" | "info" => {
            let name = if app.active_resource == "tsdb" {
                "TSDB"
            } else {
                "Info"
            };
            lines.push(section(name));
            lines.push(key_line("j / k", "Scroll"));
            lines.push(key_line("R", "Refresh"));
            lines.push(key_line("a", "Toggle auto-refresh"));
            lines.push(Line::from(""));
        }
        _ => {}
    }

    // Navigation (always shown)
    lines.push(section("Navigation"));
    lines.push(key_line("j / Down", "Move down"));
    lines.push(key_line("k / Up", "Move up"));
    lines.push(key_line("gg / Home", "Go to top"));
    lines.push(key_line("G / End", "Go to bottom"));
    lines.push(Line::from(""));

    // General (always shown)
    lines.push(section("General"));
    lines.push(key_line(":", "Command palette"));
    lines.push(key_line("/", "Filter / search"));
    lines.push(key_line("Esc", "Clear filter / cancel"));
    lines.push(key_line("?", "Toggle this help"));
    lines.push(key_line("q", "Quit"));
    lines.push(key_line("Ctrl+c", "Force quit"));

    let block = Block::default()
        .title(" Help ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn section(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        format!("  {}", title),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )])
}

fn key_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("    "),
        Span::styled(
            format!("{:>15}", key),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
