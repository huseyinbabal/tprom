use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if let Some(ref action) = app.pending_action {
        let area = centered_rect(40, 20, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" Confirm ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);

        // Question
        let question = Paragraph::new(Line::from(vec![Span::styled(
            format!("Delete \"{}\"?", action.key),
            Style::default().fg(Color::White),
        )]));
        f.render_widget(question, chunks[0]);

        // Yes / No buttons
        let yes_style = if action.selected_yes {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let no_style = if !action.selected_yes {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let buttons = Paragraph::new(Line::from(vec![
            Span::styled("  [ Yes ] ", yes_style),
            Span::raw("   "),
            Span::styled("  [ No ] ", no_style),
        ]));
        f.render_widget(buttons, chunks[2]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
