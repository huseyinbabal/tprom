use crate::app::{App, DescribeData};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    match &app.describe_data {
        DescribeData::Target(target) => {
            lines.push(section_header("Target Details"));
            lines.push(Line::from(""));
            lines.push(kv_line("Job", target.job()));
            lines.push(kv_line("Instance", target.instance()));
            lines.push(kv_line("Health", &target.health.to_uppercase()));
            lines.push(kv_line("Scrape URL", &target.scrape_url));
            lines.push(kv_line("Scrape Pool", &target.scrape_pool));
            lines.push(kv_line(
                "Last Scrape Duration",
                &format!("{:.3}s", target.last_scrape_duration),
            ));
            lines.push(kv_line("Last Scrape", &target.last_scrape));
            if !target.last_error.is_empty() {
                lines.push(kv_line("Last Error", &target.last_error));
            }
            lines.push(Line::from(""));

            // Labels
            lines.push(section_header("Labels"));
            lines.push(Line::from(""));
            let mut labels: Vec<_> = target.labels.iter().collect();
            labels.sort_by_key(|(k, _)| (*k).clone());
            for (k, v) in labels {
                lines.push(kv_line(k, v));
            }
            lines.push(Line::from(""));

            // Discovered Labels
            lines.push(section_header("Discovered Labels"));
            lines.push(Line::from(""));
            let mut dlabels: Vec<_> = target.discovered_labels.iter().collect();
            dlabels.sort_by_key(|(k, _)| (*k).clone());
            for (k, v) in dlabels {
                lines.push(kv_line(k, v));
            }
        }
        DescribeData::Alert(alert) => {
            lines.push(section_header("Alert Details"));
            lines.push(Line::from(""));
            lines.push(kv_line("Name", alert.name()));
            lines.push(kv_line("State", &alert.state.to_uppercase()));
            lines.push(kv_line("Active At", &alert.active_at));
            lines.push(kv_line("Value", &alert.value));
            lines.push(Line::from(""));

            // Labels
            lines.push(section_header("Labels"));
            lines.push(Line::from(""));
            let mut labels: Vec<_> = alert.labels.iter().collect();
            labels.sort_by_key(|(k, _)| (*k).clone());
            for (k, v) in labels {
                lines.push(kv_line(k, v));
            }
            lines.push(Line::from(""));

            // Annotations
            lines.push(section_header("Annotations"));
            lines.push(Line::from(""));
            let mut annotations: Vec<_> = alert.annotations.iter().collect();
            annotations.sort_by_key(|(k, _)| (*k).clone());
            for (k, v) in annotations {
                lines.push(kv_line(k, v));
            }
        }
        DescribeData::Rule(rule) => {
            lines.push(section_header("Rule Details"));
            lines.push(Line::from(""));
            lines.push(kv_line("Name", &rule.name));
            lines.push(kv_line("Type", &rule.rule_type));
            lines.push(kv_line("Health", &rule.health));
            lines.push(kv_line("Query", &rule.query));
            if rule.duration > 0.0 {
                lines.push(kv_line("Duration", &format!("{:.1}s", rule.duration)));
            }
            lines.push(kv_line(
                "Eval Time",
                &format!("{:.6}s", rule.evaluation_time),
            ));
            lines.push(kv_line("Last Evaluation", &rule.last_evaluation));
            lines.push(Line::from(""));

            if !rule.labels.is_empty() {
                lines.push(section_header("Labels"));
                lines.push(Line::from(""));
                let mut labels: Vec<_> = rule.labels.iter().collect();
                labels.sort_by_key(|(k, _)| (*k).clone());
                for (k, v) in labels {
                    lines.push(kv_line(k, v));
                }
                lines.push(Line::from(""));
            }

            if !rule.annotations.is_empty() {
                lines.push(section_header("Annotations"));
                lines.push(Line::from(""));
                let mut annotations: Vec<_> = rule.annotations.iter().collect();
                annotations.sort_by_key(|(k, _)| (*k).clone());
                for (k, v) in annotations {
                    lines.push(kv_line(k, v));
                }
                lines.push(Line::from(""));
            }

            if !rule.alerts.is_empty() {
                lines.push(section_header(&format!(
                    "Active Alerts ({})",
                    rule.alerts.len()
                )));
                lines.push(Line::from(""));
                for alert in &rule.alerts {
                    lines.push(Line::from(vec![
                        Span::styled("  ".to_string(), Style::default()),
                        Span::styled(
                            alert.state.to_uppercase(),
                            match alert.state.as_str() {
                                "firing" => Style::default().fg(Color::Red),
                                "pending" => Style::default().fg(Color::Yellow),
                                _ => Style::default().fg(Color::Green),
                            },
                        ),
                        Span::styled(
                            format!("  {}", alert.name()),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                }
            }
        }
        DescribeData::Metric {
            name,
            info,
            results,
        } => {
            lines.push(section_header("Metric Details"));
            lines.push(Line::from(""));
            lines.push(kv_line("Name", name));
            lines.push(kv_line("Type", &info.metric_type));
            if !info.help.is_empty() {
                lines.push(kv_line("Help", &info.help));
            }
            lines.push(Line::from(""));

            lines.push(section_header(&format!(
                "Current Values ({})",
                results.len()
            )));
            lines.push(Line::from(""));
            for result in results {
                let label_str = result.label_string();
                let value = result.instant_value().to_string();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<50}", label_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(value, Style::default().fg(Color::White)),
                ]));
            }
        }
        DescribeData::None => {
            lines.push(Line::from(Span::styled(
                "Nothing selected.".to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let visible_lines: Vec<Line> = lines.into_iter().skip(app.describe_scroll).collect();

    let title = match &app.describe_data {
        DescribeData::Target(t) => format!(" Target: {} ", t.instance()),
        DescribeData::Alert(a) => format!(" Alert: {} ", a.name()),
        DescribeData::Rule(r) => format!(" Rule: {} ", r.name),
        DescribeData::Metric { name, .. } => format!(" Metric: {} ", name),
        DescribeData::None => " Describe ".to_string(),
    };

    let paragraph = Paragraph::new(visible_lines).block(
        Block::default()
            .title(title)
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
            .fg(Color::Rgb(255, 140, 0))
            .add_modifier(Modifier::BOLD),
    ))
}

fn kv_line(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<25}", key),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}
