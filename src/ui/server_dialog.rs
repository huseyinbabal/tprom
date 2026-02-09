use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogField {
    Name,
    Url,
}

pub struct ServerDialogState {
    pub name: String,
    pub url: String,
    pub active_field: DialogField,
    pub error: Option<String>,
}

impl ServerDialogState {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            url: String::new(),
            active_field: DialogField::Name,
            error: None,
        }
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }

    pub fn clear(&mut self) {
        self.name.clear();
        self.url.clear();
        self.active_field = DialogField::Name;
        self.error = None;
    }
}

pub fn render(f: &mut Frame, state: &ServerDialogState) {
    let area = centered_rect(50, 40, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Prometheus Server ")
        .title_style(
            Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(255, 140, 0)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Name label
            Constraint::Length(3), // Name input
            Constraint::Length(1), // URL label
            Constraint::Length(3), // URL input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // error
            Constraint::Length(1), // spacer
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // Name label
    let name_label = Paragraph::new(Line::from(Span::styled(
        "Name:",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(name_label, chunks[0]);

    // Name input
    let name_style = if state.active_field == DialogField::Name {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(name_style);
    let name_input = Paragraph::new(Line::from(Span::styled(
        &state.name,
        Style::default().fg(Color::White),
    )))
    .block(name_block);
    f.render_widget(name_input, chunks[1]);

    // URL label
    let url_label = Paragraph::new(Line::from(Span::styled(
        "URL:",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(url_label, chunks[2]);

    // URL input
    let url_style = if state.active_field == DialogField::Url {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(url_style);
    let url_input = Paragraph::new(Line::from(Span::styled(
        &state.url,
        Style::default().fg(Color::White),
    )))
    .block(url_block);
    f.render_widget(url_input, chunks[3]);

    // Error message
    if let Some(ref err) = state.error {
        let error = Paragraph::new(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(error, chunks[5]);
    }

    // Hint
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("<Tab>", Style::default().fg(Color::Yellow)),
        Span::styled(" switch field  ", Style::default().fg(Color::DarkGray)),
        Span::styled("<Enter>", Style::default().fg(Color::Yellow)),
        Span::styled(" save  ", Style::default().fg(Color::DarkGray)),
        Span::styled("<Esc>", Style::default().fg(Color::Yellow)),
        Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(hint, chunks[7]);
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
