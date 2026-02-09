use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Build Info section
    if let Some(ref bi) = app.build_info {
        lines.push(section_header("# Build Info"));
        lines.push(Line::from(""));
        lines.push(kv_line("Version", &bi.version));
        lines.push(kv_line("Branch", &bi.branch));
        lines.push(kv_line("Revision", &bi.revision));
        lines.push(kv_line("Go Version", &bi.go_version));
        lines.push(kv_line("Build User", &bi.build_user));
        lines.push(kv_line("Build Date", &bi.build_date));
        lines.push(Line::from(""));
    }

    // Runtime Info section
    if let Some(ref ri) = app.runtime_info {
        lines.push(section_header("# Runtime Info"));
        lines.push(Line::from(""));
        lines.push(kv_line("Start Time", &ri.start_time));
        lines.push(kv_line("CWD", &ri.cwd));
        lines.push(kv_line(
            "Reload Config Success",
            &ri.reload_config_success.to_string(),
        ));
        lines.push(kv_line("Last Config Time", &ri.last_config_time));
        lines.push(kv_line(
            "Corruption Count",
            &ri.corruption_count.to_string(),
        ));
        lines.push(kv_line("Goroutine Count", &ri.goroutine_count.to_string()));
        lines.push(kv_line("GOMAXPROCS", &ri.gomaxprocs.to_string()));
        lines.push(kv_line("GOGC", &ri.gogc));
        lines.push(kv_line("GODEBUG", &ri.godebug));
        lines.push(kv_line("Storage Retention", &ri.storage_retention));
        lines.push(Line::from(""));
    }

    // Flags section
    if !app.flags.is_empty() {
        lines.push(section_header("# Flags"));
        lines.push(Line::from(""));
        for (key, value) in &app.flags {
            lines.push(kv_line(key, value));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No info available. Press <R> to refresh.".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let visible_lines: Vec<Line> = lines.into_iter().skip(app.info_scroll).collect();

    let paragraph = Paragraph::new(visible_lines).block(
        Block::default()
            .title(" Info ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(paragraph, area);
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn kv_line(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<30}", key),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}
