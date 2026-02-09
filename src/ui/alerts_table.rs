use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
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

            let value = if alert.value.len() > 20 {
                format!("{}...", &alert.value[..17])
            } else {
                alert.value.clone()
            };

            Row::new(vec![
                Cell::from(alert.name().to_string()),
                Cell::from(alert.state.to_uppercase()).style(state_style),
                Cell::from(active_at.to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(value).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Percentage(35),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Percentage(25),
        ratatui::layout::Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
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
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_alert_index));
    f.render_stateful_widget(table, area, &mut state);
}
