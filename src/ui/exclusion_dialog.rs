use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

// ── Dialog field enum ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExclusionField {
    Pattern,
    Servers,
}

// ── Dialog state ─────────────────────────────────────────────────────────────

pub struct ExclusionDialogState {
    pub pattern: String,
    /// All available server names (populated from TpromConfig.servers)
    pub available_servers: Vec<String>,
    /// Which servers are selected (by index into available_servers)
    pub selected_servers: Vec<bool>,
    /// Cursor position in the server list (0 = "Select All", 1.. = servers)
    pub server_cursor: usize,
    pub active_field: ExclusionField,
    pub error: Option<String>,
    /// Index of the exclusion being edited (None = creating new)
    pub editing_index: Option<usize>,
}

impl ExclusionDialogState {
    pub fn new(server_names: &[String]) -> Self {
        let selected = vec![true; server_names.len()]; // default: all selected
        Self {
            pattern: String::new(),
            available_servers: server_names.to_vec(),
            selected_servers: selected,
            server_cursor: 0,
            active_field: ExclusionField::Pattern,
            error: None,
            editing_index: None,
        }
    }

    pub fn clear(&mut self) {
        self.pattern.clear();
        self.selected_servers = vec![true; self.available_servers.len()];
        self.server_cursor = 0;
        self.active_field = ExclusionField::Pattern;
        self.error = None;
        self.editing_index = None;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }

    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            ExclusionField::Pattern => ExclusionField::Servers,
            ExclusionField::Servers => ExclusionField::Pattern,
        };
    }

    /// Toggle the server at the current cursor position.
    /// Cursor 0 = "Select All" toggle, 1.. = individual server.
    pub fn toggle_server(&mut self) {
        if self.server_cursor == 0 {
            // Select All / Deselect All
            let all_selected = self.all_selected();
            for s in self.selected_servers.iter_mut() {
                *s = !all_selected;
            }
        } else {
            let idx = self.server_cursor - 1;
            if idx < self.selected_servers.len() {
                self.selected_servers[idx] = !self.selected_servers[idx];
            }
        }
    }

    pub fn all_selected(&self) -> bool {
        self.selected_servers.iter().all(|&s| s)
    }

    pub fn none_selected(&self) -> bool {
        self.selected_servers.iter().all(|&s| !s)
    }

    /// Total rows in the server list (1 "Select All" + N servers)
    pub fn server_list_len(&self) -> usize {
        1 + self.available_servers.len()
    }

    pub fn server_cursor_down(&mut self) {
        if self.server_cursor < self.server_list_len() - 1 {
            self.server_cursor += 1;
        }
    }

    pub fn server_cursor_up(&mut self) {
        if self.server_cursor > 0 {
            self.server_cursor -= 1;
        }
    }

    /// Returns the list of selected server names.
    /// If all are selected, returns an empty vec (meaning "all servers").
    pub fn selected_server_names(&self) -> Vec<String> {
        if self.all_selected() {
            return Vec::new(); // empty = applies to all
        }
        self.available_servers
            .iter()
            .zip(self.selected_servers.iter())
            .filter_map(|(name, &sel)| if sel { Some(name.clone()) } else { None })
            .collect()
    }

    /// Load an existing exclusion for editing
    pub fn load_for_edit(&mut self, index: usize, pattern: &str, servers: &[String]) {
        self.editing_index = Some(index);
        self.pattern = pattern.to_string();
        if servers.is_empty() {
            // "all servers"
            self.selected_servers = vec![true; self.available_servers.len()];
        } else {
            self.selected_servers = self
                .available_servers
                .iter()
                .map(|name| servers.contains(name))
                .collect();
        }
        self.server_cursor = 0;
        self.active_field = ExclusionField::Pattern;
        self.error = None;
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, state: &ExclusionDialogState) {
    let area = centered_rect(55, 65, f.area());
    f.render_widget(Clear, area);

    let title = if state.editing_index.is_some() {
        " Edit Alert Exclusion "
    } else {
        " Add Alert Exclusion "
    };

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(255, 140, 0)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Server list height: "Select All" + each server, capped at 8 rows
    let server_rows = (1 + state.available_servers.len()).min(8) as u16;

    let constraints = vec![
        Constraint::Length(1),           // Pattern label
        Constraint::Length(3),           // Pattern input
        Constraint::Length(1),           // Servers label
        Constraint::Length(server_rows), // Server list
        Constraint::Length(1),           // spacer
        Constraint::Length(1),           // error
        Constraint::Length(1),           // spacer
        Constraint::Length(1),           // hint
        Constraint::Min(0),              // fill
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Pattern label
    let pattern_label = Paragraph::new(Line::from(Span::styled(
        "Regex pattern (matched against alert name):",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(pattern_label, chunks[0]);

    // Pattern input
    let pattern_style = if state.active_field == ExclusionField::Pattern {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let pattern_block = Block::default()
        .borders(Borders::ALL)
        .border_style(pattern_style);

    let cursor = if state.active_field == ExclusionField::Pattern {
        "_"
    } else {
        ""
    };
    let pattern_input = Paragraph::new(Line::from(vec![
        Span::styled(&state.pattern, Style::default().fg(Color::White)),
        Span::styled(
            cursor,
            Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::SLOW_BLINK),
        ),
    ]))
    .block(pattern_block);
    f.render_widget(pattern_input, chunks[1]);

    // Servers label
    let servers_label_style = if state.active_field == ExclusionField::Servers {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let servers_label = Paragraph::new(Line::from(Span::styled(
        "Servers (Space to toggle, j/k to navigate):",
        servers_label_style,
    )));
    f.render_widget(servers_label, chunks[2]);

    // Server list
    let server_area = chunks[3];
    let total_rows = state.server_list_len();

    // Scrolling: keep cursor visible within the server_rows window
    let visible = server_rows as usize;
    let scroll_offset = if state.server_cursor >= visible {
        state.server_cursor - visible + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();

    for i in scroll_offset..total_rows.min(scroll_offset + visible) {
        let is_cursor = i == state.server_cursor && state.active_field == ExclusionField::Servers;

        if i == 0 {
            // "Select All" row
            let checkbox = if state.all_selected() {
                "[x]"
            } else if state.none_selected() {
                "[ ]"
            } else {
                "[-]"
            };
            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };
            lines.push(Line::from(Span::styled(
                format!("  {} Select All", checkbox),
                style,
            )));
        } else {
            let idx = i - 1;
            let checkbox = if state.selected_servers[idx] {
                "[x]"
            } else {
                "[ ]"
            };
            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                format!("  {} {}", checkbox, state.available_servers[idx]),
                style,
            )));
        }
    }

    let server_list = Paragraph::new(lines);
    f.render_widget(server_list, server_area);

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
        Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
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
