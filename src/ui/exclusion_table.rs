use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.tprom_config.alert_exclusions.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No alert exclusions. Press <x> to add one.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(" Alert Exclusions ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Pattern", "Servers"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .tprom_config
        .alert_exclusions
        .iter()
        .map(|exc| {
            let servers = if exc.servers.is_empty() {
                "All servers".to_string()
            } else {
                exc.servers.join(", ")
            };
            Row::new(vec![
                Cell::from(exc.pattern.clone()).style(Style::default().fg(Color::White)),
                Cell::from(servers).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [Constraint::Percentage(50), Constraint::Percentage(50)];

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
                .title(format!(
                    " Alert Exclusions ({}) ",
                    app.tprom_config.alert_exclusions.len()
                ))
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
    state.select(Some(app.selected_exclusion_index));
    f.render_stateful_widget(table, area, &mut state);
}
