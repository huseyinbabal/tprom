use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.targets.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No targets found. Press <R> to refresh.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        "Job",
        "Instance",
        "State",
        "Last Scrape",
        "Duration",
        "Error",
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let filtered = app.apply_target_filter();

    let rows: Vec<Row> = filtered
        .iter()
        .map(|target| {
            let health_style = match target.health.as_str() {
                "up" => Style::default().fg(Color::Green),
                "down" => Style::default().fg(Color::Red),
                _ => Style::default().fg(Color::Yellow),
            };

            let health_str = target.health.to_uppercase();
            let duration = format!("{:.3}s", target.last_scrape_duration);
            let last_scrape = if target.last_scrape.len() > 19 {
                &target.last_scrape[..19]
            } else {
                &target.last_scrape
            };
            let error = if target.last_error.is_empty() {
                "-".to_string()
            } else {
                let e = &target.last_error;
                if e.len() > 40 {
                    format!("{}...", &e[..37])
                } else {
                    e.clone()
                }
            };

            let error_style = if target.last_error.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Red)
            };

            Row::new(vec![
                Cell::from(target.job().to_string()),
                Cell::from(target.instance().to_string()),
                Cell::from(health_str).style(health_style),
                Cell::from(last_scrape.to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(duration).style(Style::default().fg(Color::DarkGray)),
                Cell::from(error).style(error_style),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Percentage(30),
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
                .title(format!(" Targets ({}) ", filtered.len()))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_target_index));
    f.render_stateful_widget(table, area, &mut state);
}
