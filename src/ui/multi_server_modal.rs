use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, f.area());
    f.render_widget(Clear, area);

    let servers = &app.tprom_config.servers;
    let total = servers.len();

    // +1 for "Select All" row at the top
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title / hint
            Constraint::Min(1),    // Server list
        ])
        .split(area);

    // Title block
    let selected_count = app.multi_server_selected.len();
    let title = format!(" Multi-Server ({}/{} selected) ", selected_count, total);
    let hint_block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let hint_line = Line::from(vec![
        Span::styled("<Space>", Style::default().fg(Color::Yellow)),
        Span::styled(" Toggle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("<Enter>", Style::default().fg(Color::Yellow)),
        Span::styled(" Confirm  ", Style::default().fg(Color::DarkGray)),
        Span::styled("<Esc>", Style::default().fg(Color::Yellow)),
        Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
    ]);
    let hint = Paragraph::new(hint_line).block(hint_block);
    f.render_widget(hint, chunks[0]);

    // Server list block
    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = list_block.inner(chunks[1]);
    let inner_height = inner.height as usize;

    // Build lines: first "Select All", then each server
    let mut lines: Vec<Line> = Vec::with_capacity(total + 1);

    // Select All row
    let all_selected = !servers.is_empty() && app.multi_server_selected.len() == servers.len();
    let all_check = if all_selected { "[x]" } else { "[ ]" };
    let all_style = if app.multi_server_cursor == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} ", all_check),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("Select All", all_style),
    ]));

    // Server rows
    for (i, server) in servers.iter().enumerate() {
        let cursor_idx = i + 1; // offset by 1 for "Select All"
        let is_selected = app.multi_server_selected.contains(&server.name);
        let check = if is_selected { "[x]" } else { "[ ]" };
        let is_current = app
            .current_server
            .as_ref()
            .map(|s| s.name == server.name)
            .unwrap_or(false);

        let name_style = if app.multi_server_cursor == cursor_idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let mut spans = vec![
            Span::styled(format!("  {} ", check), Style::default().fg(Color::Yellow)),
            Span::styled(&server.name, name_style),
        ];

        if is_current {
            spans.push(Span::styled(
                " (current)",
                Style::default().fg(Color::DarkGray),
            ));
        }

        spans.push(Span::styled(
            format!("  {}", server.url),
            Style::default().fg(Color::DarkGray),
        ));

        lines.push(Line::from(spans));
    }

    // Scroll offset
    let scroll_offset = if app.multi_server_cursor >= inner_height {
        app.multi_server_cursor - inner_height + 1
    } else {
        0
    };

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll_offset)
        .take(inner_height)
        .collect();

    let list = Paragraph::new(visible_lines).block(list_block);
    f.render_widget(list, chunks[1]);
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
