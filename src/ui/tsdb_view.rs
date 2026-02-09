use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let tsdb = match &app.tsdb_status {
        Some(t) => t,
        None => {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No TSDB data. Press <R> to refresh.",
                Style::default().fg(Color::DarkGray),
            )));
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Head stats section
    lines.push(section_header("# Head Stats"));
    lines.push(Line::from(""));
    lines.push(kv_line(
        "Num Series",
        &tsdb.head_stats.num_series.to_string(),
    ));
    lines.push(kv_line(
        "Num Label Pairs",
        &tsdb.head_stats.num_label_pairs.to_string(),
    ));
    lines.push(kv_line(
        "Chunk Count",
        &tsdb.head_stats.chunk_count.to_string(),
    ));
    lines.push(kv_line("Min Time", &tsdb.head_stats.min_time.to_string()));
    lines.push(kv_line("Max Time", &tsdb.head_stats.max_time.to_string()));
    lines.push(Line::from(""));

    // Top series by metric name
    lines.push(section_header("# Top Series by Metric Name"));
    lines.push(Line::from(""));
    for stat in tsdb.series_count_by_metric_name.iter().take(10) {
        lines.push(kv_line(&stat.name, &stat.value.to_string()));
    }
    lines.push(Line::from(""));

    // Top label value counts
    lines.push(section_header("# Top Label Value Count by Label Name"));
    lines.push(Line::from(""));
    for stat in tsdb.label_value_count_by_label_name.iter().take(10) {
        lines.push(kv_line(&stat.name, &stat.value.to_string()));
    }
    lines.push(Line::from(""));

    // Memory by label name
    lines.push(section_header("# Memory by Label Name (bytes)"));
    lines.push(Line::from(""));
    for stat in tsdb.memory_in_bytes_by_label_name.iter().take(10) {
        lines.push(kv_line(&stat.name, &stat.value.to_string()));
    }
    lines.push(Line::from(""));

    // Series count by label value pair
    lines.push(section_header("# Top Series by Label Value Pair"));
    lines.push(Line::from(""));
    for stat in tsdb.series_count_by_label_value_pair.iter().take(10) {
        lines.push(kv_line(&stat.name, &stat.value.to_string()));
    }

    // Apply scroll
    let visible_lines: Vec<Line> = lines.into_iter().skip(app.tsdb_scroll).collect();

    let paragraph = Paragraph::new(visible_lines).block(
        Block::default()
            .title(" TSDB Status ")
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
            format!("  {:<35}", key),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}
