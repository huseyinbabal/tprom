use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.filtered_metrics.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No metrics found. Press <R> to refresh.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Metric Name", "Type", "Help"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered_metrics
        .iter()
        .map(|name| {
            let (metric_type, help) = if let Some(metas) = app.metadata.get(name) {
                if let Some(m) = metas.first() {
                    (m.metric_type.clone(), m.help.clone())
                } else {
                    ("unknown".to_string(), String::new())
                }
            } else {
                ("unknown".to_string(), String::new())
            };

            let type_style = match metric_type.as_str() {
                "counter" => Style::default().fg(Color::Green),
                "gauge" => Style::default().fg(Color::Blue),
                "histogram" => Style::default().fg(Color::Magenta),
                "summary" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::DarkGray),
            };

            let help_truncated = if help.len() > 60 {
                format!("{}...", &help[..57])
            } else {
                help
            };

            Row::new(vec![
                Cell::from(name.clone()),
                Cell::from(metric_type).style(type_style),
                Cell::from(help_truncated).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Percentage(40),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Percentage(50),
    ];

    let title = if app.filter_text.is_empty() {
        format!(" Metrics ({}) ", app.metric_names.len())
    } else {
        format!(
            " Metrics ({}/{}) ",
            app.filtered_metrics.len(),
            app.metric_names.len()
        )
    };

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
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_metric_index));
    f.render_stateful_widget(table, area, &mut state);
}
