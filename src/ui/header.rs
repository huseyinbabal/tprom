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
        .spacing(2)
        .constraints([
            Constraint::Fill(1),    // Context info
            Constraint::Length(16), // Stats
            Constraint::Length(23), // Keybindings 1
            Constraint::Length(22), // Keybindings 2
            Constraint::Length(24), // Logo
        ])
        .split(area);

    render_context_column(f, app, columns[0]);
    render_stats_column(f, app, columns[1]);
    render_keybindings_col1(f, app, columns[2]);
    render_keybindings_col2(f, columns[3]);
    render_logo(f, columns[4]);
}

/// Truncate `s` to at most `max` chars, ending with an ellipsis when cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max == 0 {
        String::new()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn render_context_column(f: &mut Frame, app: &App, area: Rect) {
    let multi_alerts = app.multi_server_mode_active && app.active_resource == "alerts";
    let server = if multi_alerts {
        format!("{} servers", app.multi_server_alerts.len())
    } else {
        app.current_server_name().to_string()
    };
    let value_width = (area.width as usize).saturating_sub(9);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Server:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&server, value_width),
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

    let (refresh, refresh_style) = if app.auto_refresh_enabled {
        (
            format!(" every {}s", app.auto_refresh_interval.as_secs()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (" off".to_string(), Style::default().fg(Color::DarkGray))
    };
    let sound = if app.tprom_config.alert_sound {
        Span::styled("  \u{266a}", Style::default().fg(Color::Magenta)) // ♪
    } else {
        Span::raw("")
    };
    lines.push(Line::from(vec![
        Span::styled("Refresh: ", Style::default().fg(Color::DarkGray)),
        Span::styled(refresh, refresh_style),
        sound,
    ]));

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
            let alerts: Vec<&crate::model::Alert> = if app.multi_server_mode_active {
                app.multi_server_alerts
                    .iter()
                    .flat_map(|(_, a)| a.iter())
                    .collect()
            } else {
                app.alerts.iter().collect()
            };
            let total = alerts.len();
            let firing = alerts.iter().filter(|a| a.state == "firing").count();
            let pending = alerts.iter().filter(|a| a.state == "pending").count();
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
        "query" => {
            vec![
                Line::from(vec![
                    Span::styled("Results: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.query_results.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Type:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if app.query_result_type.is_empty() {
                            "-".to_string()
                        } else {
                            app.query_result_type.clone()
                        },
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("History: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.query_history.len().to_string(),
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
        "targets" | "rules" | "metrics" => vec![
            ("<d>", "Describe"),
            ("</>", "Filter"),
            ("<R>", "Refresh"),
            ("<a>", "Auto-refresh"),
        ],
        "alerts" => vec![
            ("<d>", "Describe"),
            ("</>", "Filter"),
            ("<x>", "Exclude"),
            ("<m>", "Multi-Server"),
        ],
        "alertexclusion" => vec![("<x>", "Add"), ("<d>", "Edit"), ("<D>", "Delete"), ("", "")],
        "labels" => vec![
            ("<Tab>", "Switch panel"),
            ("<R>", "Refresh"),
            ("<a>", "Auto-refresh"),
            ("", ""),
        ],
        "config" => vec![
            ("<j/k>", "Scroll"),
            ("</>", "Search"),
            ("<n/N>", "Next/Prev"),
            ("<R>", "Refresh"),
        ],
        "query" => vec![
            ("<Tab>", "Switch focus"),
            ("<Enter>", "Execute"),
            ("<v>", "Graph"),
            ("<m>", "Multi-Server"),
        ],
        "tsdb" | "info" => vec![
            ("<j/k>", "Scroll"),
            ("<R>", "Refresh"),
            ("<a>", "Auto-refresh"),
            ("", ""),
        ],
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
        ("<?>", "Help"),
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
    let logo_style = Style::default()
        .fg(Color::Rgb(255, 140, 0))
        .add_modifier(Modifier::BOLD);
    let logo = vec![
        Line::from(Span::styled("▀█▀ █▀▀█ █▀▀▄ █▀▀█ █▀▄▀█", logo_style)),
        Line::from(Span::styled(" █  █▄▄▀ █▀▀▄ █  █ █ █ █", logo_style)),
        Line::from(Span::styled(" ▀  ▀    ▀  ▀ ▀▀▀▀ ▀   ▀", logo_style)),
        Line::from(Span::styled(
            "  Prometheus TUI",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!("       v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(logo).alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}
