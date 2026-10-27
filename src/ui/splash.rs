use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub struct SplashState {
    pub spinner_frame: usize,
    pub message: String,
}

impl SplashState {
    pub fn new() -> Self {
        Self {
            spinner_frame: 0,
            message: "Connecting to Prometheus...".to_string(),
        }
    }
}

pub fn render(f: &mut Frame, state: &SplashState) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(10),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(area);

    // Logo - block letter TPROM
    let logo_style = Style::default()
        .fg(Color::Rgb(255, 140, 0))
        .add_modifier(Modifier::BOLD);
    let logo = vec![
        Line::from(Span::styled(
            "████████╗██████╗ ██████╗  ██████╗ ███╗   ███╗",
            logo_style,
        )),
        Line::from(Span::styled(
            "╚══██╔══╝██╔══██╗██╔══██╗██╔═══██╗████╗ ████║",
            logo_style,
        )),
        Line::from(Span::styled(
            "   ██║   ██████╔╝██████╔╝██║   ██║██╔████╔██║",
            logo_style,
        )),
        Line::from(Span::styled(
            "   ██║   ██╔═══╝ ██╔══██╗██║   ██║██║╚██╔╝██║",
            logo_style,
        )),
        Line::from(Span::styled(
            "   ██║   ██║     ██║  ██║╚██████╔╝██║ ╚═╝ ██║",
            logo_style,
        )),
        Line::from(Span::styled(
            "   ╚═╝   ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝",
            logo_style,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Terminal UI for Prometheus",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let logo_widget = Paragraph::new(logo).alignment(Alignment::Center);
    f.render_widget(logo_widget, chunks[1]);

    // Spinner + message
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸"];
    let spinner = spinner_chars[state.spinner_frame % spinner_chars.len()];
    let status = Line::from(vec![
        Span::styled(
            format!("{} ", spinner),
            Style::default().fg(Color::Rgb(255, 140, 0)),
        ),
        Span::styled(&state.message, Style::default().fg(Color::DarkGray)),
    ]);

    let status_widget = Paragraph::new(status).alignment(Alignment::Center);
    f.render_widget(status_widget, chunks[2]);
}
