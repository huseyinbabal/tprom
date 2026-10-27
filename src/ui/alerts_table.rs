use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use super::format_alert_value;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if app.multi_server_mode_active {
        render_multi_server(f, app, area);
    } else {
        render_single_server(f, app, area);
    }
}

fn render_single_server(f: &mut Frame, app: &mut App, area: Rect) {
    if app.alerts.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No alerts found. Press <R> to refresh.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Alert Name", "State", "Active Since", "Value"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let filtered = app.apply_alert_filter();

    let rows: Vec<Row> = filtered
        .iter()
        .map(|alert| {
            let state_style = match alert.state.as_str() {
                "firing" => Style::default().fg(Color::Red),
                "pending" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::Green),
            };

            let active_at = if alert.active_at.len() > 19 {
                &alert.active_at[..19]
            } else {
                &alert.active_at
            };

            let value = format_alert_value(&alert.value);
            let value = if value.len() > 20 {
                format!("{}...", &value[..17])
            } else {
                value
            };

            Row::new(vec![
                Cell::from(alert.name().to_string()),
                Cell::from(alert.state.to_uppercase()).style(state_style),
                Cell::from(active_at.to_string()).style(Style::default().fg(Color::Gray)),
                Cell::from(value).style(Style::default().fg(Color::Gray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(30),
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Length(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 64, 90))
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(format!(" Alerts ({}) ", filtered.len()))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    app.alert_table_state.select(Some(app.selected_alert_index));
    f.render_stateful_widget(table, area, &mut app.alert_table_state);
}

fn render_multi_server(f: &mut Frame, app: &mut App, area: Rect) {
    let filtered = app.filtered_multi_server_alerts();
    let total_alerts: usize = app.multi_server_alerts.iter().map(|(_, a)| a.len()).sum();
    let filtered_total: usize = filtered.iter().map(|(_, a)| a.len()).sum();

    if app.multi_server_alerts.is_empty() || total_alerts == 0 {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No alerts found from selected servers. Press <m> to select servers, <R> to refresh.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    if filtered.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            format!(
                "No alerts matching '{}'. Press <Esc> to clear filter.",
                app.filter_text
            ),
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(" Alerts - Multi-Server (filtered) ")
                .title_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Alert Name", "State", "Active Since", "Value"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    // Build flat rows from filtered data (respecting collapsed state)
    let mut rows: Vec<Row> = Vec::new();

    for (server_name, alerts) in &filtered {
        let is_collapsed = app.multi_server_collapsed_alerts.contains(server_name);
        let indicator = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" }; // ▶ or ▼
        let firing = alerts.iter().filter(|a| a.state == "firing").count();
        let pending = alerts.iter().filter(|a| a.state == "pending").count();

        let count_cell = |n: usize, label: &str, color: Color| {
            if n > 0 {
                Cell::from(format!("{} {}", n, label))
                    .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            } else {
                Cell::from(format!("0 {}", label)).style(Style::default().fg(Color::DarkGray))
            }
        };

        rows.push(Row::new(vec![
            Cell::from(Line::from(vec![
                Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    server_name.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ({})", alerts.len()),
                    Style::default().fg(Color::Gray),
                ),
            ])),
            count_cell(firing, "firing", Color::Red),
            count_cell(pending, "pending", Color::Yellow),
            Cell::from(""),
        ]));

        // Only render children if expanded
        if !is_collapsed {
            for alert in alerts {
                let state_style = match alert.state.as_str() {
                    "firing" => Style::default().fg(Color::Red),
                    "pending" => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::Green),
                };

                let active_at = if alert.active_at.len() > 19 {
                    &alert.active_at[..19]
                } else {
                    &alert.active_at
                };

                let value = format_alert_value(&alert.value);
                let value = if value.len() > 20 {
                    format!("{}...", &value[..17])
                } else {
                    value
                };

                rows.push(Row::new(vec![
                    Cell::from(Line::from(vec![
                        Span::styled("  \u{2502} ", Style::default().fg(Color::DarkGray)), // │
                        Span::styled(alert.name().to_string(), Style::default().fg(Color::White)),
                    ])),
                    Cell::from(alert.state.to_uppercase()).style(state_style),
                    Cell::from(active_at.to_string()).style(Style::default().fg(Color::Gray)),
                    Cell::from(value).style(Style::default().fg(Color::Gray)),
                ]));
            }
        }
    }

    let widths = [
        Constraint::Min(30),
        Constraint::Length(12),
        Constraint::Length(20),
        Constraint::Length(20),
    ];

    let title = if app.filter_text.is_empty() {
        format!(
            " Alerts - Multi-Server ({} alerts, {} servers) | h/l collapse/expand  E/W all ",
            total_alerts,
            app.multi_server_alerts.len()
        )
    } else {
        format!(
            " Alerts - Multi-Server ({}/{} matched, {} servers) | /: {} ",
            filtered_total,
            total_alerts,
            filtered.len(),
            app.filter_text
        )
    };

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 64, 90))
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    app.multi_alert_table_state
        .select(Some(app.multi_server_flat_alert_index));
    f.render_stateful_widget(table, area, &mut app.multi_alert_table_state);
}
