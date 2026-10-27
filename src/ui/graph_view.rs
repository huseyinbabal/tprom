use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

/// A palette of distinct colors for multiple series
const SERIES_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::LightRed,
    Color::LightBlue,
    Color::LightGreen,
    Color::White,
];

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    // One legend entry per row
    let legend_height = (app.graph_data.len() as u16).max(1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),             // Title / info bar
            Constraint::Min(1),                // Chart area
            Constraint::Length(legend_height), // Legend (one entry per line)
        ])
        .split(area);

    render_title(f, app, chunks[0]);
    render_chart(f, app, chunks[1]);
    render_legend(f, app, chunks[2]);
}

fn render_title(f: &mut Frame, app: &App, area: Rect) {
    let expr = app.query_input.trim();
    let series_count = app.graph_data.len();

    let title = format!(
        " Graph: {} ({} series, last 1h) ",
        if expr.len() > 50 {
            format!("{}...", &expr[..47])
        } else {
            expr.to_string()
        },
        series_count
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(255, 140, 0)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Rgb(255, 140, 0))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(
            " Esc",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" close  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "←/→",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" highlight series", Style::default().fg(Color::DarkGray)),
    ]))
    .block(block);

    f.render_widget(hint, area);
}

fn render_chart(f: &mut Frame, app: &App, area: Rect) {
    if app.graph_data.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" No data ")
            .title_alignment(Alignment::Center);
        let msg = Paragraph::new(Line::from(Span::styled(
            "No time series data returned for this query",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(msg, area);
        return;
    }

    // Format time axis labels
    let time_min = app.graph_time_min;
    let time_max = app.graph_time_max;
    let start_label = format_timestamp(time_min);
    let mid_label = format_timestamp((time_min + time_max) / 2.0);
    let end_label = format_timestamp(time_max);

    // Format value axis labels
    let val_min_label = format_value(app.graph_value_min);
    let val_mid_label = format_value((app.graph_value_min + app.graph_value_max) / 2.0);
    let val_max_label = format_value(app.graph_value_max);

    // Build datasets — each series gets its own color.
    // When a legend item is hovered, dim all other series.
    let hovered = app.graph_legend_hovered;
    let datasets: Vec<Dataset> = app
        .graph_data
        .iter()
        .enumerate()
        .map(|(i, (label, points))| {
            let base_color = SERIES_COLORS[i % SERIES_COLORS.len()];
            let color = match hovered {
                Some(h) if h != i => Color::DarkGray,
                _ => base_color,
            };
            let short_label = if label.len() > 30 {
                format!("{}...", &label[..27])
            } else {
                label.clone()
            };
            Dataset::default()
                .name(short_label)
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(color))
                .data(points)
        })
        .collect();

    let x_axis = Axis::default()
        .title(Span::styled("Time", Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::DarkGray))
        .bounds([time_min, time_max])
        .labels(vec![
            Span::from(start_label),
            Span::from(mid_label),
            Span::from(end_label),
        ]);

    let y_axis = Axis::default()
        .title(Span::styled("Value", Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::DarkGray))
        .bounds([app.graph_value_min, app.graph_value_max])
        .labels(vec![
            Span::from(val_min_label),
            Span::from(val_mid_label),
            Span::from(val_max_label),
        ]);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    f.render_widget(chart, area);
}

fn render_legend(f: &mut Frame, app: &App, area: Rect) {
    let hovered = app.graph_legend_hovered;

    let lines: Vec<Line> = app
        .graph_data
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let base_color = SERIES_COLORS[i % SERIES_COLORS.len()];
            let is_hovered = hovered == Some(i);

            let marker_style = if is_hovered {
                Style::default().fg(base_color).add_modifier(Modifier::BOLD)
            } else if hovered.is_some() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(base_color)
            };

            let short = if label.len() > 50 {
                format!("{}...", &label[..47])
            } else {
                label.clone()
            };

            let label_style = if is_hovered {
                Style::default()
                    .fg(base_color)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else if hovered.is_some() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![
                Span::styled("━━ ", marker_style),
                Span::styled(short, label_style),
            ])
        })
        .collect();

    let legend = Paragraph::new(lines);
    f.render_widget(legend, area);
}

fn format_timestamp(ts: f64) -> String {
    let secs = ts as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0);
    match dt {
        Some(d) => d.format("%H:%M").to_string(),
        None => format!("{:.0}", ts),
    }
}

fn format_value(val: f64) -> String {
    if val.abs() >= 1_000_000.0 {
        format!("{:.1}M", val / 1_000_000.0)
    } else if val.abs() >= 1_000.0 {
        format!("{:.1}K", val / 1_000.0)
    } else if val == val.floor() {
        format!("{:.0}", val)
    } else {
        format!("{:.2}", val)
    }
}
