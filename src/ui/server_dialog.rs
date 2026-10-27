use crate::model::AuthConfig;
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
    AuthType,
    AuthUser,
    AuthSecret,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
}

impl AuthType {
    pub fn label(&self) -> &'static str {
        match self {
            AuthType::None => "None",
            AuthType::Basic => "Basic (username:password)",
            AuthType::Bearer => "Bearer Token",
        }
    }

    pub fn next(&self) -> AuthType {
        match self {
            AuthType::None => AuthType::Basic,
            AuthType::Basic => AuthType::Bearer,
            AuthType::Bearer => AuthType::None,
        }
    }

    pub fn prev(&self) -> AuthType {
        match self {
            AuthType::None => AuthType::Bearer,
            AuthType::Basic => AuthType::None,
            AuthType::Bearer => AuthType::Basic,
        }
    }
}

pub struct ServerDialogState {
    pub name: String,
    pub url: String,
    pub auth_type: AuthType,
    pub auth_user: String,
    pub auth_secret: String, // password for Basic, token for Bearer
    pub active_field: DialogField,
    pub error: Option<String>,
}

impl ServerDialogState {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            url: String::new(),
            auth_type: AuthType::None,
            auth_user: String::new(),
            auth_secret: String::new(),
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
        self.auth_type = AuthType::None;
        self.auth_user.clear();
        self.auth_secret.clear();
        self.active_field = DialogField::Name;
        self.error = None;
    }

    pub fn get_auth_config(&self) -> AuthConfig {
        match self.auth_type {
            AuthType::None => AuthConfig::None,
            AuthType::Basic => AuthConfig::Basic {
                username: self.auth_user.clone(),
                password: self.auth_secret.clone(),
            },
            AuthType::Bearer => AuthConfig::Bearer {
                token: self.auth_secret.clone(),
            },
        }
    }

    /// Cycle to the next Tab-able field
    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            DialogField::Name => DialogField::Url,
            DialogField::Url => DialogField::AuthType,
            DialogField::AuthType => match self.auth_type {
                AuthType::None => DialogField::Name,
                AuthType::Basic => DialogField::AuthUser,
                AuthType::Bearer => DialogField::AuthSecret,
            },
            DialogField::AuthUser => DialogField::AuthSecret,
            DialogField::AuthSecret => DialogField::Name,
        };
    }
}

pub fn render(f: &mut Frame, state: &ServerDialogState) {
    let area = centered_rect(55, 55, f.area());
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

    let auth_field_count = match state.auth_type {
        AuthType::None => 0,
        AuthType::Basic => 4,  // user label + input + pass label + input
        AuthType::Bearer => 2, // token label + input
    };

    let mut constraints = vec![
        Constraint::Length(1), // Name label
        Constraint::Length(3), // Name input
        Constraint::Length(1), // URL label
        Constraint::Length(3), // URL input
        Constraint::Length(1), // Auth type label
        Constraint::Length(1), // Auth type selector
    ];

    for _ in 0..auth_field_count {
        constraints.push(Constraint::Length(if auth_field_count > 0 { 2 } else { 0 }));
    }

    constraints.push(Constraint::Length(1)); // spacer
    constraints.push(Constraint::Length(1)); // error
    constraints.push(Constraint::Length(1)); // spacer
    constraints.push(Constraint::Length(1)); // hint
    constraints.push(Constraint::Min(0)); // fill

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
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

    // Auth type label
    let auth_label = Paragraph::new(Line::from(Span::styled(
        "Auth:",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(auth_label, chunks[4]);

    // Auth type selector
    let auth_style = if state.active_field == DialogField::AuthType {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let arrow_hint = if state.active_field == DialogField::AuthType {
        " (Left/Right to change)"
    } else {
        ""
    };
    let auth_selector = Paragraph::new(Line::from(vec![
        Span::styled(format!("  {}", state.auth_type.label()), auth_style),
        Span::styled(arrow_hint, Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(auth_selector, chunks[5]);

    // Auth-specific fields
    let mut idx = 6;
    match state.auth_type {
        AuthType::None => {}
        AuthType::Basic => {
            // Username
            let user_label = Paragraph::new(Line::from(Span::styled(
                "Username:",
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(user_label, chunks[idx]);
            idx += 1;

            let user_style = if state.active_field == DialogField::AuthUser {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let user_input = Paragraph::new(Line::from(Span::styled(
                &state.auth_user,
                Style::default().fg(Color::White),
            )))
            .style(user_style);
            f.render_widget(user_input, chunks[idx]);
            idx += 1;

            // Password
            let pass_label = Paragraph::new(Line::from(Span::styled(
                "Password:",
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(pass_label, chunks[idx]);
            idx += 1;

            let pass_style = if state.active_field == DialogField::AuthSecret {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let masked: String = "*".repeat(state.auth_secret.len());
            let pass_input = Paragraph::new(Line::from(Span::styled(
                masked,
                Style::default().fg(Color::White),
            )))
            .style(pass_style);
            f.render_widget(pass_input, chunks[idx]);
            idx += 1;
        }
        AuthType::Bearer => {
            // Token
            let token_label = Paragraph::new(Line::from(Span::styled(
                "Token:",
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(token_label, chunks[idx]);
            idx += 1;

            let token_style = if state.active_field == DialogField::AuthSecret {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            // Show first 8 chars + masked rest
            let display = if state.auth_secret.len() <= 8 {
                state.auth_secret.clone()
            } else {
                format!(
                    "{}{}",
                    &state.auth_secret[..8],
                    "*".repeat(state.auth_secret.len() - 8)
                )
            };
            let token_input = Paragraph::new(Line::from(Span::styled(
                display,
                Style::default().fg(Color::White),
            )))
            .style(token_style);
            f.render_widget(token_input, chunks[idx]);
            idx += 1;
        }
    }

    // Error message (idx is after auth fields)
    let error_idx = idx + 1; // skip spacer
    if let Some(ref err) = state.error {
        if error_idx < chunks.len() {
            let error = Paragraph::new(Line::from(Span::styled(
                err.as_str(),
                Style::default().fg(Color::Red),
            )));
            f.render_widget(error, chunks[error_idx]);
        }
    }

    // Hint (idx + 3: after spacer, error, spacer)
    let hint_idx = idx + 3;
    if hint_idx < chunks.len() {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("<Tab>", Style::default().fg(Color::Yellow)),
            Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
            Span::styled("<Enter>", Style::default().fg(Color::Yellow)),
            Span::styled(" save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("<Esc>", Style::default().fg(Color::Yellow)),
            Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
        ]));
        f.render_widget(hint, chunks[hint_idx]);
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
