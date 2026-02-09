use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.flat_rules.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No rules found. Press <R> to refresh.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Group", "Name", "Type", "Health", "Last Eval"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let filtered = app.apply_rule_filter();

    let rows: Vec<Row> = filtered
        .iter()
        .map(|(group_name, rule)| {
            let health_style = match rule.health.as_str() {
                "ok" => Style::default().fg(Color::Green),
                "err" => Style::default().fg(Color::Red),
                _ => Style::default().fg(Color::Yellow),
            };

            let type_style = match rule.rule_type.as_str() {
                "alerting" => Style::default().fg(Color::Rgb(255, 140, 0)),
                "recording" => Style::default().fg(Color::Blue),
                _ => Style::default().fg(Color::White),
            };

            let last_eval = if rule.last_evaluation.len() > 19 {
                &rule.last_evaluation[..19]
            } else {
                &rule.last_evaluation
            };

            Row::new(vec![
                Cell::from(group_name.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(rule.name.clone()),
                Cell::from(rule.rule_type.clone()).style(type_style),
                Cell::from(rule.health.clone()).style(health_style),
                Cell::from(last_eval.to_string()).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(30),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Percentage(20),
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
                .title(format!(" Rules ({}) ", filtered.len()))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_rule_index));
    f.render_stateful_widget(table, area, &mut state);
}
