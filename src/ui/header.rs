use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Context info
            Constraint::Percentage(25), // Stats
            Constraint::Percentage(20), // Keybindings 1
            Constraint::Percentage(20), // Keybindings 2
            Constraint::Percentage(15), // Logo
        ])
        .split(area);

    render_context_column(f, app, columns[0]);
    render_stats_column(f, app, columns[1]);
    render_keybindings_col1(f, app, columns[2]);
    render_keybindings_col2(f, columns[3]);
    render_logo(f, columns[4]);
}

fn render_context_column(f: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Server:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.current_server_name(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Resource:", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" {}", app.active_resource),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn render_stats_column(f: &mut Frame, app: &App, area: Rect) {
    let lines = match app.active_resource.as_str() {
        "targets" => {
            let total = app.targets.len();
            let up = app.targets.iter().filter(|t| t.health == "up").count();
            let down = total - up;
            vec![
                Line::from(vec![
                    Span::styled("Total:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        total.to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Up:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        up.to_string(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Down:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        down.to_string(),
                        Style::default()
                            .fg(if down > 0 { Color::Red } else { Color::White })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]
        }
        "alerts" => {
            let total = app.alerts.len();
            let firing = app.alerts.iter().filter(|a| a.state == "firing").count();
            let pending = app.alerts.iter().filter(|a| a.state == "pending").count();
            vec![
                Line::from(vec![
                    Span::styled("Total:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        total.to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Firing:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        firing.to_string(),
                        Style::default()
                            .fg(if firing > 0 { Color::Red } else { Color::White })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Pending: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        pending.to_string(),
                        Style::default()
                            .fg(if pending > 0 {
                                Color::Yellow
                            } else {
                                Color::White
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]
        }
        "metrics" => {
            vec![
                Line::from(vec![
                    Span::styled("Total:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.metric_names.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Showing: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.filtered_metrics.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]
        }
        "rules" => {
            vec![
                Line::from(vec![
                    Span::styled("Groups:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.rule_groups.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Rules:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.flat_rules.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]
        }
        "labels" => {
            vec![
                Line::from(vec![
                    Span::styled("Labels:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.label_names.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Values:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.label_values.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ]
        }
        _ => {
            vec![Line::from("")]
        }
    };

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn render_keybindings_col1(f: &mut Frame, app: &App, area: Rect) {
    let bindings = match app.active_resource.as_str() {
        "servers" => vec![
            ("<c>", "Connect"),
            ("<a>", "Add"),
            ("<C-d>", "Delete"),
            ("", ""),
        ],
        "targets" | "alerts" | "rules" | "metrics" => vec![
            ("<d>", "Describe"),
            ("</>", "Filter"),
            ("<R>", "Refresh"),
            ("", ""),
        ],
        "labels" => vec![
            ("<Tab>", "Switch panel"),
            ("<R>", "Refresh"),
            ("", ""),
            ("", ""),
        ],
        "config" => vec![
            ("<j/k>", "Scroll"),
            ("</>", "Search"),
            ("<n/N>", "Next/Prev"),
            ("<R>", "Refresh"),
        ],
        "tsdb" | "info" => vec![("<j/k>", "Scroll"), ("<R>", "Refresh"), ("", ""), ("", "")],
        _ => vec![
            ("<j/k>", "Navigate"),
            ("<R>", "Refresh"),
            ("", ""),
            ("", ""),
        ],
    };

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                Line::from("")
            } else {
                Line::from(vec![
                    Span::styled(format!("{:<9}", key), Style::default().fg(Color::Yellow)),
                    Span::styled(*desc, Style::default().fg(Color::DarkGray)),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn render_keybindings_col2(f: &mut Frame, area: Rect) {
    let bindings = [
        ("<:>", "Command"),
        ("<q>", "Quit"),
        ("<ctrl-c>", "Force Quit"),
    ];

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!("{:<10}", key), Style::default().fg(Color::Yellow)),
                Span::styled(*desc, Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn render_logo(f: &mut Frame, area: Rect) {
    let logo = vec![
        Line::from(Span::styled(
            "▀█▀ █▀█ █▀█ █▀█ █▄█",
            Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " █  █▀▀ █▀▄ █▄█ █ █",
            Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "      Prometheus TUI",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!("            v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(logo).alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}
