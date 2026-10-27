use super::format_alert_value;
use crate::app::{App, AutocompleteKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Query input
            Constraint::Min(1),    // Results
        ])
        .split(area);

    render_input(f, app, chunks[0]);

    if app.multi_server_mode_active {
        render_multi_server_results(f, app, chunks[1]);
    } else {
        render_results(f, app, chunks[1]);
    }

    // Render autocomplete overlay on top of results
    if app.query_input_active
        && app.autocomplete_visible
        && !app.autocomplete_suggestions.is_empty()
    {
        render_autocomplete(f, app, chunks[0], chunks[1]);
    }
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.query_input_active {
        Color::Rgb(255, 140, 0) // Orange when active
    } else {
        Color::DarkGray
    };

    let title = if app.query_input_active {
        if app.autocomplete_visible {
            " PromQL (Tab to accept, Esc to dismiss) "
        } else {
            " PromQL (Enter to execute, Tab for results) "
        }
    } else {
        " PromQL (Tab to edit, v graph, d describe) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);

    let input_text = if app.query_input_active {
        Line::from(vec![
            Span::styled(&app.query_input, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Rgb(255, 140, 0))),
        ])
    } else {
        Line::from(Span::styled(
            &app.query_input,
            Style::default().fg(Color::White),
        ))
    };

    let input = Paragraph::new(input_text).block(block);
    f.render_widget(input, area);
}

fn render_autocomplete(f: &mut Frame, app: &App, input_area: Rect, results_area: Rect) {
    let suggestion_count = app.autocomplete_suggestions.len().min(10);
    let dropdown_height = (suggestion_count as u16) + 2; // +2 for borders

    // Position the dropdown right below the input box, aligned to the left
    let dropdown_width = results_area.width.min(60);
    let dropdown = Rect {
        x: input_area.x + 1,
        y: input_area.y + input_area.height,
        width: dropdown_width,
        height: dropdown_height.min(results_area.height),
    };

    // Clear the area behind the dropdown
    f.render_widget(Clear, dropdown);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(255, 140, 0)));

    let rows: Vec<Row> = app
        .autocomplete_suggestions
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.autocomplete_selected;

            let kind_color = match item.kind {
                AutocompleteKind::Metric => Color::Cyan,
                AutocompleteKind::Function => Color::Magenta,
                AutocompleteKind::Keyword => Color::Yellow,
                AutocompleteKind::LabelName => Color::Green,
                AutocompleteKind::Operator => Color::LightRed,
                AutocompleteKind::LabelValue => Color::LightBlue,
            };

            let text_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    format!(" {:<4}", item.kind.label()),
                    Style::default().fg(kind_color),
                )),
                Cell::from(Span::styled(&item.text, text_style)),
            ])
        })
        .collect();

    let widths = [Constraint::Length(6), Constraint::Min(10)];

    let table = Table::new(rows, widths).block(block).row_highlight_style(
        Style::default()
            .bg(Color::Rgb(60, 60, 60))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = TableState::default();
    state.select(Some(app.autocomplete_selected));
    f.render_stateful_widget(table, dropdown, &mut state);
}

fn render_results(f: &mut Frame, app: &App, area: Rect) {
    if app.query_results.is_empty() {
        let msg = if app.query_input.is_empty() && app.query_result_type.is_empty() {
            "Type a PromQL expression and press Enter to execute"
        } else if app.query_result_type.is_empty() {
            "No query executed yet"
        } else {
            "No results"
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Results ",
                Style::default().fg(Color::DarkGray),
            ))
            .title_alignment(Alignment::Center);

        let paragraph = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let result_border_color = if !app.query_input_active {
        Color::Rgb(255, 140, 0)
    } else {
        Color::DarkGray
    };

    let title = format!(
        " Results: {} ({} series) ",
        app.query_result_type,
        app.query_results.len()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(result_border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(result_border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);

    let header = Row::new(vec![
        Cell::from(Span::styled(
            "#",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Metric",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Value",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .query_results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let label_str = result.label_string();
            let value = format_alert_value(result.instant_value());

            Row::new(vec![
                Cell::from(Span::styled(
                    format!("{}", i + 1),
                    Style::default().fg(Color::DarkGray),
                )),
                Cell::from(Span::styled(label_str, Style::default().fg(Color::Cyan))),
                Cell::from(Span::styled(
                    value,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(5),
        Constraint::Percentage(70),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 40))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_query_result_index));
    f.render_stateful_widget(table, area, &mut state);
}

fn render_multi_server_results(f: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_multi_server_query();
    let total_results: usize = app
        .multi_server_query_results
        .iter()
        .map(|(_, r)| r.len())
        .sum();
    let filtered_total: usize = filtered.iter().map(|(_, r)| r.len()).sum();

    if app.multi_server_query_results.is_empty() || total_results == 0 {
        let msg = if app.query_input.is_empty() {
            "Type a PromQL expression and press Enter to execute across selected servers"
        } else {
            "No results from selected servers. Press <m> to select servers."
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Multi-Server Results ",
                Style::default().fg(Color::DarkGray),
            ))
            .title_alignment(Alignment::Center);

        let paragraph = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(paragraph, area);
        return;
    }

    if filtered.is_empty() && !app.filter_text.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(Span::styled(
                " Results - Multi-Server (filtered) ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let paragraph = Paragraph::new(Line::from(Span::styled(
            format!(
                "No results matching '{}'. Press <Esc> to clear filter.",
                app.filter_text
            ),
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let result_border_color = if !app.query_input_active {
        Color::Magenta
    } else {
        Color::DarkGray
    };

    let title = if app.filter_text.is_empty() {
        format!(
            " Results - Multi-Server: {} ({} series, {} servers) | \u{2190}/\u{2192} collapse/expand ",
            app.multi_server_query_result_type,
            total_results,
            app.multi_server_query_results.len()
        )
    } else {
        format!(
            " Results - Multi-Server: {} ({}/{} matched, {} servers) | /: {} ",
            app.multi_server_query_result_type,
            filtered_total,
            total_results,
            filtered.len(),
            app.filter_text
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(result_border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(result_border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);

    let header = Row::new(vec![
        Cell::from(Span::styled(
            "",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "#",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Metric",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Value",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .height(1);

    // Build flat rows from filtered data (respecting collapsed state)
    let mut rows: Vec<Row> = Vec::new();

    for (server_name, results) in &filtered {
        let is_collapsed = app.multi_server_collapsed_query.contains(server_name);
        let indicator = if is_collapsed { "\u{25b6}" } else { "\u{25bc}" }; // ▶ or ▼
        let server_label = format!(" {} {} ({} series)", indicator, server_name, results.len());

        rows.push(
            Row::new(vec![
                Cell::from(""),
                Cell::from(""),
                Cell::from(Span::styled(
                    server_label,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(""),
            ])
            .style(Style::default().bg(Color::Rgb(50, 30, 70))),
        );

        // Only render children if expanded
        if !is_collapsed {
            for (i, result) in results.iter().enumerate() {
                let label_str = result.label_string();
                let value = format_alert_value(result.instant_value());

                rows.push(Row::new(vec![
                    Cell::from(Span::styled("  ", Style::default().fg(Color::DarkGray))),
                    Cell::from(Span::styled(
                        format!("{}", i + 1),
                        Style::default().fg(Color::DarkGray),
                    )),
                    Cell::from(Span::styled(
                        format!("  {}", label_str),
                        Style::default().fg(Color::Cyan),
                    )),
                    Cell::from(Span::styled(
                        value,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                ]));
            }
        }
    }

    let widths = [
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Percentage(65),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 40))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    state.select(Some(app.multi_server_flat_query_index));
    f.render_stateful_widget(table, area, &mut state);
}
