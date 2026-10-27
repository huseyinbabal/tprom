use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.tprom_config.servers.is_empty() {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled(
                "No servers configured. Press ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("<a>", Style::default().fg(Color::Yellow)),
            Span::styled(" to add one.", Style::default().fg(Color::DarkGray)),
        ]));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["#", "Name", "URL"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .tprom_config
        .servers
        .iter()
        .enumerate()
        .map(|(i, server)| {
            let is_current = app
                .current_server
                .as_ref()
                .map(|s| s.name == server.name)
                .unwrap_or(false);

            let indicator = if is_current { "*" } else { " " };

            Row::new(vec![
                Cell::from(format!("{}{}", indicator, i + 1)),
                Cell::from(server.name.clone()),
                Cell::from(server.url.clone()),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(4),
        ratatui::layout::Constraint::Percentage(30),
        ratatui::layout::Constraint::Percentage(65),
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
                .title(" Servers ")
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
    state.select(Some(app.selected_server_index));
    f.render_stateful_widget(table, area, &mut state);
}
