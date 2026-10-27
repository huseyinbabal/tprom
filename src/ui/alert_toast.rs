use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Persistent notification box for newly firing alerts, pinned to the
/// top-right corner of `area`. Stays until dismissed with Esc.
pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.alert_toasts.is_empty() || area.width < 20 || area.height < 5 {
        return;
    }

    let width = 72.min(area.width.saturating_sub(2));
    let inner_width = width.saturating_sub(4) as usize;
    // Each toast takes two lines; keep room for borders and the hint line
    let max_entries = (area.height.saturating_sub(4) / 2).clamp(1, 6) as usize;
    let shown = app.alert_toasts.len().min(max_entries);
    let hidden = app.alert_toasts.len() - shown;

    let mut lines: Vec<Line> = Vec::new();
    for toast in app.alert_toasts.iter().take(shown) {
        let severity_color = match toast.severity.as_str() {
            "critical" => Color::Red,
            "warning" => Color::Yellow,
            _ => Color::Cyan,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", toast.at.format("%H:%M:%S")),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                truncate(&toast.alertname, inner_width.saturating_sub(9)),
                Style::default()
                    .fg(severity_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        let detail = if toast.summary.is_empty() {
            toast.server.clone()
        } else {
            format!("{} · {}", toast.server, toast.summary)
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", truncate(&detail, inner_width.saturating_sub(2))),
            Style::default().fg(Color::White),
        )));
    }

    let mut hint = vec![Span::styled("Esc", Style::default().fg(Color::Yellow))];
    hint.push(Span::styled(" dismiss", Style::default().fg(Color::Gray)));
    if hidden > 0 {
        hint.push(Span::styled(
            format!("  ·  +{} more", hidden),
            Style::default().fg(Color::Gray),
        ));
    }
    lines.push(Line::from(hint));

    let height = (lines.len() as u16 + 2).min(area.height);
    let toast_area = Rect {
        x: area.x + area.width.saturating_sub(width + 1),
        y: area.y,
        width,
        height,
    };

    let count = app.alert_toasts.len();
    let title = format!(
        " \u{25cf} {} new firing alert{} ",
        count,
        if count == 1 { "" } else { "s" }
    );

    f.render_widget(Clear, toast_area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .title_alignment(Alignment::Left)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .style(Style::default().bg(Color::Rgb(30, 18, 22))),
        ),
        toast_area,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max == 0 {
        String::new()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
