use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Label names
            Constraint::Percentage(60), // Label values
        ])
        .split(area);

    render_label_names(f, app, chunks[0]);
    render_label_values(f, app, chunks[1]);
}

fn render_label_names(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if !app.label_focus_right {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    if app.label_names.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No labels. Press <R>.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(" Labels ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = app
        .label_names
        .iter()
        .map(|name| Row::new(vec![name.clone()]))
        .collect();

    let widths = [Constraint::Percentage(100)];

    let table = Table::new(rows, widths)
        .row_highlight_style(
            Style::default()
                .bg(if !app.label_focus_right {
                    Color::DarkGray
                } else {
                    Color::Rgb(40, 40, 40)
                })
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(format!(" Labels ({}) ", app.label_names.len()))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_label_index));
    f.render_stateful_widget(table, area, &mut state);
}

fn render_label_values(f: &mut Frame, app: &App, area: Rect) {
    let label_name = if app.selected_label_index < app.label_names.len() {
        &app.label_names[app.selected_label_index]
    } else {
        ""
    };

    let border_color = if app.label_focus_right {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    if app.label_values.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No values",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(format!(" Values for \"{}\" ", label_name))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = app
        .label_values
        .iter()
        .map(|value| Row::new(vec![value.clone()]))
        .collect();

    let widths = [Constraint::Percentage(100)];

    let table = Table::new(rows, widths)
        .row_highlight_style(
            Style::default()
                .bg(if app.label_focus_right {
                    Color::DarkGray
                } else {
                    Color::Rgb(40, 40, 40)
                })
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(format!(
                    " Values for \"{}\" ({}) ",
                    label_name,
                    app.label_values.len()
                ))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );

    let mut state = TableState::default();
    state.select(Some(app.selected_label_value_index));
    f.render_stateful_widget(table, area, &mut state);
}
