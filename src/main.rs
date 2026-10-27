#![allow(dead_code)]

mod app;
mod client;
mod model;
mod notify;
mod ui;

use anyhow::Result;
use app::{App, DescribeData, Mode, PendingAction, PendingActionType};
use clap::Parser;
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use model::*;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use ui::server_dialog::DialogField;

pub const VERSION: &str = match option_env!("TPROM_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// Events sent from async tasks back to the main loop
#[allow(dead_code)]
#[allow(clippy::large_enum_variant)] // short-lived channel messages; boxing buys nothing
enum AppEvent {
    Progress(String),
    Connect,
    Connected,
    Error(String),
    /// Background fetch completed with result
    FetchDone(FetchResult),
}

/// Result of a background HTTP fetch
#[allow(dead_code)]
enum FetchResult {
    /// Initial connection: health check + alerts + metrics + labels
    ConnectDone {
        build_info: BuildInfoData,
        alerts: Vec<Alert>,
        metric_names: Vec<String>,
        metadata: HashMap<String, Vec<MetricMetadata>>,
        label_names: Vec<String>,
    },
    ConnectFailed(String),
    Targets(Vec<ActiveTarget>),
    Alerts(Vec<Alert>),
    Rules {
        groups: Vec<RuleGroup>,
        flat_rules: Vec<(String, Rule)>,
    },
    Metrics {
        names: Vec<String>,
        metadata: HashMap<String, Vec<MetricMetadata>>,
    },
    Labels {
        names: Vec<String>,
        values: Vec<String>,
    },
    TsdbStatus(TsdbStatusData),
    Config(String),
    Info {
        build_info: Option<BuildInfoData>,
        runtime_info: Option<RuntimeInfoData>,
        flags: Vec<(String, String)>,
    },
    Query {
        result_type: String,
        results: Vec<QueryResult>,
    },
    GraphQuery {
        graph_data: Vec<(String, Vec<(f64, f64)>)>,
        time_min: f64,
        time_max: f64,
        value_min: f64,
        value_max: f64,
    },
    DescribeMetric {
        data: DescribeData,
    },
    LabelValues {
        values: Vec<String>,
    },
    ResourceError {
        resource: String,
        error: String,
    },
    /// Multi-server alerts results (parallel fetch from multiple servers)
    /// Per server: its alerts, or `None` when the fetch failed
    MultiServerAlerts(Vec<(String, Option<Vec<Alert>>)>),
    /// Multi-server query results (parallel fetch from multiple servers)
    MultiServerQuery {
        results: Vec<(String, Vec<QueryResult>)>,
        result_type: String,
    },
}

/// TProm - Terminal UI for Prometheus
#[derive(Parser, Debug)]
#[command(name = "tprom", version = VERSION, about = "Terminal UI for Prometheus")]
pub struct Args {
    /// Prometheus server URL
    #[arg(short, long)]
    pub url: Option<String>,

    /// Auto-refresh every N seconds for this session (overrides config)
    #[arg(short, long, value_name = "SECS", value_parser = clap::value_parser!(u64).range(1..))]
    pub refresh: Option<u64>,

    /// Bearer token for authentication
    #[arg(long)]
    pub token: Option<String>,

    /// Username for basic authentication
    #[arg(long)]
    pub user: Option<String>,

    /// Password for basic authentication
    #[arg(long)]
    pub password: Option<String>,

    /// Notification sound for this session (overrides config): default,
    /// alarm, siren, klaxon, a macOS system sound name or a file path
    #[arg(long, value_name = "SOUND")]
    pub notification_sound: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.rebuild_exclusions();
    if let Some(secs) = args.refresh {
        app.auto_refresh_enabled = true;
        app.auto_refresh_interval = Duration::from_secs(secs);
    }
    app.notification_sound_override = args.notification_sound.clone();

    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);

    // Determine initial mode
    if let Some(ref url) = args.url {
        // Determine auth from CLI flags
        let auth = if let Some(ref token) = args.token {
            model::AuthConfig::Bearer {
                token: token.clone(),
            }
        } else if let Some(ref user) = args.user {
            model::AuthConfig::Basic {
                username: user.clone(),
                password: args.password.clone().unwrap_or_default(),
            }
        } else {
            model::AuthConfig::None
        };

        // CLI URL provided, use directly
        app.current_server = Some(model::ServerConfig {
            name: url.clone(),
            url: url.clone(),
            auth,
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = tx_clone
                .send(AppEvent::Progress(
                    "Connecting to Prometheus...".to_string(),
                ))
                .await;
            let _ = tx_clone.send(AppEvent::Connect).await;
        });
    } else if app.needs_server_setup() {
        app.mode = Mode::ServerDialog;
    } else {
        // Use first saved server
        let server = app.tprom_config.servers[0].clone();
        app.current_server = Some(server);

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = tx_clone
                .send(AppEvent::Progress(
                    "Connecting to Prometheus...".to_string(),
                ))
                .await;
            let _ = tx_clone.send(AppEvent::Connect).await;
        });
    }

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Process async events
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Progress(msg) => {
                    app.splash_state.message = msg;
                }
                AppEvent::Connect => {
                    if let Err(e) = app.connect() {
                        app.set_error(format!("Connection failed: {}", e));
                        app.mode = Mode::ServerDialog;
                    } else if let Some(client) = app.client.clone() {
                        // Spawn background task for connection + initial data fetch
                        let tx_bg = tx.clone();
                        tokio::spawn(async move {
                            match client.health_check().await {
                                Ok(bi) => {
                                    let alerts = client
                                        .fetch_alerts()
                                        .await
                                        .map(|d| d.alerts)
                                        .unwrap_or_default();
                                    let metric_names =
                                        client.fetch_metric_names().await.unwrap_or_default();
                                    let metadata =
                                        client.fetch_metadata().await.unwrap_or_default();
                                    let label_names =
                                        client.fetch_label_names().await.unwrap_or_default();
                                    let _ = tx_bg
                                        .send(AppEvent::FetchDone(FetchResult::ConnectDone {
                                            build_info: bi,
                                            alerts,
                                            metric_names,
                                            metadata,
                                            label_names,
                                        }))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_bg
                                        .send(AppEvent::FetchDone(FetchResult::ConnectFailed(
                                            format!("{}", e),
                                        )))
                                        .await;
                                }
                            }
                        });
                    }
                }
                AppEvent::Connected => {
                    app.mode = Mode::Normal;
                }
                AppEvent::Error(msg) => {
                    app.set_error(msg);
                }
                AppEvent::FetchDone(result) => {
                    app.loading = false;
                    match result {
                        FetchResult::ConnectDone {
                            build_info,
                            alerts,
                            metric_names,
                            metadata,
                            label_names,
                        } => {
                            app.build_info = Some(build_info);
                            app.alerts = alerts;
                            app.check_new_firing_alerts(&[]);
                            let filtered_len = app.apply_alert_filter().len();
                            if filtered_len == 0 || app.selected_alert_index >= filtered_len {
                                app.selected_alert_index = 0;
                            }
                            app.metric_names = metric_names;
                            app.metadata = metadata;
                            app.apply_metric_filter();
                            app.rebuild_autocomplete_pool();
                            app.cached_label_names = label_names;
                            app.mode = Mode::Normal;
                            app.active_resource = "alerts".to_string();
                            app.query_input_active = false;
                            app.set_status("Connected successfully");
                        }
                        FetchResult::ConnectFailed(e) => {
                            app.set_error(format!("Connection failed: {}", e));
                            app.mode = Mode::ServerDialog;
                            app.client = None;
                        }
                        FetchResult::Targets(targets) => {
                            app.targets = targets;
                            let filtered_len = app.apply_target_filter().len();
                            if filtered_len == 0 || app.selected_target_index >= filtered_len {
                                app.selected_target_index = 0;
                            }
                        }
                        FetchResult::Alerts(alerts) => {
                            app.alerts = alerts;
                            app.check_new_firing_alerts(&[]);
                            let filtered_len = app.apply_alert_filter().len();
                            if filtered_len == 0 || app.selected_alert_index >= filtered_len {
                                app.selected_alert_index = 0;
                            }
                        }
                        FetchResult::Rules { groups, flat_rules } => {
                            app.rule_groups = groups;
                            app.flat_rules = flat_rules;
                            if app.selected_rule_index >= app.flat_rules.len() {
                                app.selected_rule_index = 0;
                            }
                        }
                        FetchResult::Metrics { names, metadata } => {
                            app.metric_names = names;
                            app.metadata = metadata;
                            app.apply_metric_filter();
                            app.rebuild_autocomplete_pool();
                        }
                        FetchResult::Labels { names, values } => {
                            app.label_names = names;
                            app.label_values = values;
                            app.selected_label_index = 0;
                            app.selected_label_value_index = 0;
                        }
                        FetchResult::TsdbStatus(status) => {
                            app.tsdb_status = Some(status);
                            app.tsdb_scroll = 0;
                        }
                        FetchResult::Config(yaml) => {
                            app.config_yaml = yaml;
                            app.config_scroll = 0;
                        }
                        FetchResult::Info {
                            build_info,
                            runtime_info,
                            flags,
                        } => {
                            app.build_info = build_info;
                            app.runtime_info = runtime_info;
                            app.flags = flags;
                            app.info_scroll = 0;
                        }
                        FetchResult::Query {
                            result_type,
                            results,
                        } => {
                            app.query_result_type = result_type;
                            app.query_results = results;
                            app.selected_query_result_index = 0;
                            if !app.query_results.is_empty() {
                                app.query_input_active = false;
                            }
                        }
                        FetchResult::GraphQuery {
                            graph_data,
                            time_min,
                            time_max,
                            value_min,
                            value_max,
                        } => {
                            app.graph_data = graph_data;
                            app.graph_time_min = time_min;
                            app.graph_time_max = time_max;
                            app.graph_value_min = value_min;
                            app.graph_value_max = value_max;
                            if app.graph_data.is_empty() {
                                app.set_error("No time series data to graph");
                            } else {
                                app.graph_visible = true;
                                app.set_status(format!("Graph: {} series", app.graph_data.len()));
                            }
                        }
                        FetchResult::DescribeMetric { data } => {
                            app.describe_data = data;
                            app.describe_scroll = 0;
                            app.mode = Mode::Describe;
                        }
                        FetchResult::LabelValues { values } => {
                            app.label_values = values;
                            app.selected_label_value_index = 0;
                        }
                        FetchResult::ResourceError { resource, error } => {
                            app.set_error(format!("Failed to fetch {}: {}", resource, error));
                        }
                        FetchResult::MultiServerAlerts(results) => {
                            let same_servers = app.multi_server_mode_active
                                && app.multi_server_alerts.len() == results.len()
                                && app
                                    .multi_server_alerts
                                    .iter()
                                    .all(|(name, _)| results.iter().any(|(n, _)| n == name));
                            if !same_servers {
                                // New selection: start with all server groups collapsed
                                app.multi_server_collapsed_alerts =
                                    results.iter().map(|(name, _)| name.clone()).collect();
                                app.multi_server_flat_alert_index = 0;
                            }
                            // A failed server keeps the alerts from its last successful fetch
                            let mut failed = Vec::new();
                            let merged: Vec<(String, Vec<Alert>)> = results
                                .into_iter()
                                .map(|(name, alerts)| match alerts {
                                    Some(alerts) => (name, alerts),
                                    None => {
                                        let previous = app
                                            .multi_server_alerts
                                            .iter()
                                            .find(|(n, _)| *n == name)
                                            .map(|(_, a)| a.clone())
                                            .unwrap_or_default();
                                        failed.push(name.clone());
                                        (name, previous)
                                    }
                                })
                                .collect();
                            app.multi_server_alerts = merged;
                            app.multi_server_mode_active = true;
                            app.check_new_firing_alerts(&failed);
                            let total: usize =
                                app.multi_server_alerts.iter().map(|(_, a)| a.len()).sum();
                            let mut status = format!(
                                "Multi-server: {} alerts from {} servers",
                                total,
                                app.multi_server_alerts.len()
                            );
                            if !failed.is_empty() {
                                status.push_str(&format!(
                                    " ({} unreachable: {})",
                                    failed.len(),
                                    failed.join(", ")
                                ));
                            }
                            app.set_status(status);
                        }
                        FetchResult::MultiServerQuery {
                            results,
                            result_type,
                        } => {
                            // Start collapsed: all server groups collapsed
                            app.multi_server_collapsed_query =
                                results.iter().map(|(name, _)| name.clone()).collect();
                            app.multi_server_query_results = results;
                            app.multi_server_query_result_type = result_type;
                            app.multi_server_mode_active = true;
                            app.multi_server_flat_query_index = 0;
                            let total: usize = app
                                .multi_server_query_results
                                .iter()
                                .map(|(_, r)| r.len())
                                .sum();
                            app.set_status(format!(
                                "Multi-server: {} results from {} servers",
                                total,
                                app.multi_server_query_results.len()
                            ));
                            if !app.multi_server_query_results.is_empty() {
                                app.query_input_active = false;
                            }
                        }
                    }
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                // Global quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.should_quit = true;
                }

                match app.mode {
                    Mode::Splash => {
                        // No user input during splash, just wait for connection
                    }
                    Mode::ServerDialog => {
                        handle_server_dialog_keys(&mut app, key.code, &tx).await;
                    }
                    Mode::Resources => {
                        handle_resources_keys(&mut app, key.code, &tx).await;
                    }
                    Mode::Confirm => {
                        handle_confirm_keys(&mut app, key.code);
                    }
                    Mode::MultiServerSelect => {
                        handle_multi_server_keys(&mut app, key.code, &tx);
                    }
                    Mode::ExclusionDialog => {
                        handle_exclusion_dialog_keys(&mut app, key.code);
                    }
                    Mode::Help => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                            app.mode = Mode::Normal;
                        }
                        _ => {}
                    },
                    Mode::Describe => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.mode = Mode::Normal;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.describe_scroll_down();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.describe_scroll_up();
                        }
                        KeyCode::Char('G') | KeyCode::End => {
                            app.describe_scroll = 999999;
                        }
                        KeyCode::Home => {
                            app.describe_scroll = 0;
                        }
                        _ => {}
                    },
                    Mode::Normal => {
                        if app.graph_visible {
                            // Graph mode: Esc to close, Left/Right/h/l to navigate legend
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => {
                                    if app.graph_legend_hovered.is_some() {
                                        // First Esc clears the legend highlight
                                        app.graph_legend_hovered = None;
                                    } else {
                                        app.graph_visible = false;
                                    }
                                }
                                KeyCode::Char(':') => {
                                    app.graph_visible = false;
                                    app.graph_legend_hovered = None;
                                    app.mode = Mode::Resources;
                                    app.command_text.clear();
                                    app.command_suggestion_selected = 0;
                                    app.update_command_suggestions();
                                }
                                KeyCode::Left | KeyCode::Char('h') => {
                                    if !app.graph_data.is_empty() {
                                        app.graph_legend_hovered =
                                            Some(match app.graph_legend_hovered {
                                                Some(0) => app.graph_data.len() - 1,
                                                Some(i) => i - 1,
                                                None => app.graph_data.len() - 1,
                                            });
                                    }
                                }
                                KeyCode::Right | KeyCode::Char('l')
                                    if !app.graph_data.is_empty() =>
                                {
                                    app.graph_legend_hovered =
                                        Some(match app.graph_legend_hovered {
                                            Some(i) if i >= app.graph_data.len() - 1 => 0,
                                            Some(i) => i + 1,
                                            None => 0,
                                        });
                                }
                                _ => {}
                            }
                        } else if app.filter_active {
                            handle_filter_keys(&mut app, key.code).await;
                        } else if app.config_search_active {
                            handle_config_search_keys(&mut app, key.code);
                        } else if app.active_resource == "query" && app.query_input_active {
                            // ':' on empty query input opens command palette
                            if key.code == KeyCode::Char(':') && app.query_input.is_empty() {
                                app.query_input_active = false;
                                app.mode = Mode::Resources;
                                app.command_text.clear();
                                app.command_suggestion_selected = 0;
                                app.update_command_suggestions();
                            } else {
                                handle_query_input_keys(&mut app, key.code, &tx).await;
                            }
                        } else {
                            handle_normal_keys(&mut app, key.code, &tx).await;
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            app.maybe_clear_status();
            last_tick = Instant::now();

            // Auto-refresh check
            if app.auto_refresh_enabled && app.mode == Mode::Normal && !app.loading {
                let should_refresh = match app.last_auto_refresh {
                    Some(last) => last.elapsed() >= app.auto_refresh_interval,
                    None => true,
                };
                if should_refresh {
                    app.last_auto_refresh = Some(Instant::now());
                    if app.multi_server_mode_active && app.active_resource == "alerts" {
                        spawn_multi_server_alerts(&mut app, &tx);
                    } else {
                        let resource = app.active_resource.clone();
                        spawn_fetch(&mut app, &resource, &tx);
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn handle_server_dialog_keys(app: &mut App, key: KeyCode, tx: &mpsc::Sender<AppEvent>) {
    match key {
        KeyCode::Esc => {
            if app.needs_server_setup() {
                app.should_quit = true;
            } else {
                app.mode = Mode::Normal;
                app.server_dialog_state.clear();
            }
        }
        KeyCode::Tab => {
            app.server_dialog_state.next_field();
        }
        KeyCode::Left => {
            if app.server_dialog_state.active_field == DialogField::AuthType {
                app.server_dialog_state.auth_type = app.server_dialog_state.auth_type.prev();
            }
        }
        KeyCode::Right => {
            if app.server_dialog_state.active_field == DialogField::AuthType {
                app.server_dialog_state.auth_type = app.server_dialog_state.auth_type.next();
            }
        }
        KeyCode::Enter => {
            if app.server_dialog_state.active_field == DialogField::AuthType {
                // On AuthType field, Enter cycles the type
                app.server_dialog_state.auth_type = app.server_dialog_state.auth_type.next();
            } else {
                if let Err(e) = app.add_server_from_dialog() {
                    app.server_dialog_state.set_error(format!("{}", e));
                } else if app.current_server.is_some() {
                    app.server_dialog_state.clear();
                    app.mode = Mode::Splash;
                    app.splash_state = ui::splash::SplashState::new();

                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        let _ = tx_clone
                            .send(AppEvent::Progress(
                                "Connecting to Prometheus...".to_string(),
                            ))
                            .await;
                        let _ = tx_clone.send(AppEvent::Connect).await;
                    });
                }
            }
        }
        KeyCode::Backspace => match app.server_dialog_state.active_field {
            DialogField::Name => {
                app.server_dialog_state.name.pop();
            }
            DialogField::Url => {
                app.server_dialog_state.url.pop();
            }
            DialogField::AuthType => {} // no-op
            DialogField::AuthUser => {
                app.server_dialog_state.auth_user.pop();
            }
            DialogField::AuthSecret => {
                app.server_dialog_state.auth_secret.pop();
            }
        },
        KeyCode::Char(c) => match app.server_dialog_state.active_field {
            DialogField::Name => {
                app.server_dialog_state.name.push(c);
            }
            DialogField::Url => {
                app.server_dialog_state.url.push(c);
            }
            DialogField::AuthType => {} // no-op, use arrows
            DialogField::AuthUser => {
                app.server_dialog_state.auth_user.push(c);
            }
            DialogField::AuthSecret => {
                app.server_dialog_state.auth_secret.push(c);
            }
        },
        _ => {}
    }
}

fn handle_exclusion_dialog_keys(app: &mut App, key: KeyCode) {
    use crate::ui::exclusion_dialog::ExclusionField;

    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.exclusion_dialog_state.clear();
        }
        KeyCode::Tab => {
            app.exclusion_dialog_state.next_field();
        }
        KeyCode::Enter => {
            match app.exclusion_dialog_state.active_field {
                ExclusionField::Servers => {
                    // On server list, Enter toggles the current server
                    app.exclusion_dialog_state.toggle_server();
                }
                ExclusionField::Pattern => {
                    // Save the exclusion
                    if let Err(e) = app.add_exclusion_from_dialog() {
                        app.exclusion_dialog_state.set_error(format!("{}", e));
                    } else if app.exclusion_dialog_state.error.is_none() {
                        app.exclusion_dialog_state.clear();
                        app.mode = Mode::Normal;
                    }
                }
            }
        }
        KeyCode::Char(' ') => {
            if app.exclusion_dialog_state.active_field == ExclusionField::Servers {
                app.exclusion_dialog_state.toggle_server();
            } else {
                app.exclusion_dialog_state.pattern.push(' ');
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.exclusion_dialog_state.active_field == ExclusionField::Servers {
                app.exclusion_dialog_state.server_cursor_down();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.exclusion_dialog_state.active_field == ExclusionField::Servers {
                app.exclusion_dialog_state.server_cursor_up();
            }
        }
        KeyCode::Backspace => {
            if app.exclusion_dialog_state.active_field == ExclusionField::Pattern {
                app.exclusion_dialog_state.pattern.pop();
            }
        }
        KeyCode::Char(c) if app.exclusion_dialog_state.active_field == ExclusionField::Pattern => {
            app.exclusion_dialog_state.pattern.push(c);
        }
        _ => {}
    }
}

async fn handle_resources_keys(app: &mut App, key: KeyCode, tx: &mpsc::Sender<AppEvent>) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.command_text.clear();
        }
        KeyCode::Enter => {
            if !app.command_suggestions.is_empty() {
                let selected = &app.command_suggestions[app.command_suggestion_selected];
                let cmd = selected.command.clone();
                app.active_resource = cmd.clone();
                app.mode = Mode::Normal;
                app.command_text.clear();

                // When switching to query view, start in input mode
                if cmd == "query" {
                    app.query_input_active = true;
                }

                // Dismiss any open autocomplete
                app.autocomplete_visible = false;
                app.autocomplete_suggestions.clear();

                // Fetch data for the selected resource in background
                spawn_fetch(app, &cmd, tx);
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            if !app.command_suggestions.is_empty() {
                app.command_suggestion_selected =
                    (app.command_suggestion_selected + 1) % app.command_suggestions.len();
                app.command_preview = app
                    .command_suggestions
                    .get(app.command_suggestion_selected)
                    .map(|r| r.command.clone());
            }
        }
        KeyCode::Up => {
            if !app.command_suggestions.is_empty() {
                if app.command_suggestion_selected == 0 {
                    app.command_suggestion_selected = app.command_suggestions.len() - 1;
                } else {
                    app.command_suggestion_selected -= 1;
                }
                app.command_preview = app
                    .command_suggestions
                    .get(app.command_suggestion_selected)
                    .map(|r| r.command.clone());
            }
        }
        KeyCode::Backspace => {
            app.command_text.pop();
            app.update_command_suggestions();
        }
        KeyCode::Char(c) => {
            app.command_text.push(c);
            app.update_command_suggestions();
        }
        _ => {}
    }
}

fn handle_confirm_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.pending_action = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            if let Some(ref mut action) = app.pending_action {
                action.selected_yes = !action.selected_yes;
            }
        }
        KeyCode::Enter => {
            if let Some(ref action) = app.pending_action {
                if action.selected_yes {
                    match action.action_type {
                        PendingActionType::DeleteServer => {
                            let name = action.key.clone();
                            let _ = app.delete_server(&name);
                            if app.selected_server_index >= app.tprom_config.servers.len()
                                && app.selected_server_index > 0
                            {
                                app.selected_server_index -= 1;
                            }
                        }
                        PendingActionType::DeleteExclusion => {
                            let _ = app.delete_exclusion(app.selected_exclusion_index);
                        }
                    }
                }
            }
            app.pending_action = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_multi_server_keys(app: &mut App, key: KeyCode, tx: &mpsc::Sender<AppEvent>) {
    let server_count = app.tprom_config.servers.len();
    let total_rows = server_count + 1; // +1 for "Select All"

    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.multi_server_cursor < total_rows.saturating_sub(1) {
                app.multi_server_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.multi_server_cursor > 0 {
                app.multi_server_cursor -= 1;
            }
        }
        KeyCode::Char(' ') => {
            if app.multi_server_cursor == 0 {
                // Select All
                app.multi_server_toggle_all();
            } else {
                let idx = app.multi_server_cursor - 1;
                if idx < server_count {
                    let name = app.tprom_config.servers[idx].name.clone();
                    app.multi_server_toggle(&name);
                }
            }
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            if app.multi_server_selected.is_empty() {
                // No servers selected, deactivate multi-server mode
                app.multi_server_mode_active = false;
                return;
            }
            // Trigger multi-server fetch based on active resource
            match app.active_resource.as_str() {
                "alerts" => {
                    spawn_multi_server_alerts(app, tx);
                }
                "query" => {
                    spawn_multi_server_query(app, tx);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

async fn handle_filter_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.filter_text.clear();
            app.filter_active = false;
            // Reset filter
            if app.active_resource.as_str() == "metrics" {
                app.apply_metric_filter()
            }
            // Restore collapsed state from before filter
            if app.multi_server_mode_active && app.active_resource == "alerts" {
                if let Some(saved) = app.pre_filter_collapsed_alerts.take() {
                    app.multi_server_collapsed_alerts = saved;
                }
            } else if app.multi_server_mode_active && app.active_resource == "query" {
                if let Some(saved) = app.pre_filter_collapsed_query.take() {
                    app.multi_server_collapsed_query = saved;
                }
            }
        }
        KeyCode::Enter => {
            app.filter_active = false;
            // Keep filter applied
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            if app.active_resource.as_str() == "metrics" {
                app.apply_metric_filter()
            }
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            if app.active_resource.as_str() == "metrics" {
                app.apply_metric_filter()
            }
        }
        _ => {}
    }

    // Clamp multi-server flat index after filter changes
    if app.multi_server_mode_active {
        if app.active_resource == "alerts" {
            let len = app.multi_server_alert_flat_len();
            if len == 0 {
                app.multi_server_flat_alert_index = 0;
            } else if app.multi_server_flat_alert_index >= len {
                app.multi_server_flat_alert_index = len - 1;
            }
        } else if app.active_resource == "query" {
            let len = app.multi_server_query_flat_len();
            if len == 0 {
                app.multi_server_flat_query_index = 0;
            } else if app.multi_server_flat_query_index >= len {
                app.multi_server_flat_query_index = len - 1;
            }
        }
    }
}

fn handle_config_search_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Enter => {
            app.config_search_active = false;
        }
        KeyCode::Backspace => {
            app.config_search_text.pop();
            app.update_config_search();
        }
        KeyCode::Char(c) => {
            app.config_search_text.push(c);
            app.update_config_search();
        }
        _ => {}
    }
}

async fn handle_normal_keys(app: &mut App, key: KeyCode, tx: &mpsc::Sender<AppEvent>) {
    // Esc dismisses alert notifications before anything else
    if key == KeyCode::Esc && !app.alert_toasts.is_empty() {
        app.alert_toasts.clear();
        return;
    }

    match key {
        KeyCode::Char('q') => {
            if app.active_resource == "query" && !app.query_input_active {
                app.query_input_active = true;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => app.nav_down(),
        KeyCode::Char('k') | KeyCode::Up => app.nav_up(),
        KeyCode::Char('g') => {
            if let Some((KeyCode::Char('g'), last_time)) = app.last_key_press {
                if last_time.elapsed() < Duration::from_millis(250) {
                    app.go_to_top();
                    app.last_key_press = None;
                    return;
                }
            }
            app.last_key_press = Some((KeyCode::Char('g'), Instant::now()));
        }
        KeyCode::Char('G') | KeyCode::End => app.go_to_bottom(),
        KeyCode::Home => app.go_to_top(),

        // Command palette
        KeyCode::Char(':') => {
            app.mode = Mode::Resources;
            app.command_text.clear();
            app.command_suggestion_selected = 0;
            app.update_command_suggestions();
        }

        // Filter / search
        KeyCode::Char('/') => {
            match app.active_resource.as_str() {
                "targets" | "alerts" | "rules" | "metrics" => {
                    app.filter_active = true;
                    app.filter_text.clear();
                    // Expand all collapsed groups when filtering in multi-server mode
                    if app.multi_server_mode_active && app.active_resource == "alerts" {
                        app.pre_filter_collapsed_alerts =
                            Some(app.multi_server_collapsed_alerts.clone());
                        app.multi_server_collapsed_alerts.clear();
                    }
                }
                "config" => {
                    app.config_search_active = true;
                    app.config_search_text.clear();
                    app.config_search_matches.clear();
                    app.config_search_current = 0;
                }
                "query" => {
                    if app.multi_server_mode_active && !app.query_input_active {
                        // In multi-server query results view, activate filter
                        app.filter_active = true;
                        app.filter_text.clear();
                        app.pre_filter_collapsed_query =
                            Some(app.multi_server_collapsed_query.clone());
                        app.multi_server_collapsed_query.clear();
                    } else {
                        app.query_input_active = true;
                    }
                }
                _ => {}
            }
        }

        // Search navigation (config)
        KeyCode::Char('n') => {
            if app.active_resource == "config" && !app.config_search_text.is_empty() {
                app.config_search_next();
            }
        }
        KeyCode::Char('N') => {
            if app.active_resource == "config" && !app.config_search_text.is_empty() {
                app.config_search_prev();
            }
        }

        // Describe
        KeyCode::Char('d') | KeyCode::Enter => match app.active_resource.as_str() {
            "targets" => {
                app.describe_target();
                if !matches!(app.describe_data, app::DescribeData::None) {
                    app.mode = Mode::Describe;
                }
            }
            "alerts" => {
                app.describe_alert();
                if !matches!(app.describe_data, app::DescribeData::None) {
                    app.mode = Mode::Describe;
                }
            }
            "rules" => {
                app.describe_rule();
                if !matches!(app.describe_data, app::DescribeData::None) {
                    app.mode = Mode::Describe;
                }
            }
            "metrics" => {
                spawn_describe_metric(app, tx);
            }
            "query" => {
                if !app.query_input_active {
                    app.describe_query_result();
                    if !matches!(app.describe_data, app::DescribeData::None) {
                        app.mode = Mode::Describe;
                    }
                }
            }
            "alertexclusion" if !app.tprom_config.alert_exclusions.is_empty() => {
                let exc = &app.tprom_config.alert_exclusions[app.selected_exclusion_index];
                let server_names: Vec<String> = app
                    .tprom_config
                    .servers
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                app.exclusion_dialog_state =
                    ui::exclusion_dialog::ExclusionDialogState::new(&server_names);
                app.exclusion_dialog_state.load_for_edit(
                    app.selected_exclusion_index,
                    &exc.pattern.clone(),
                    &exc.servers.clone(),
                );
                app.mode = Mode::ExclusionDialog;
            }
            _ => {}
        },

        // Graph (visualize)
        KeyCode::Char('v') => {
            if app.active_resource == "query" && !app.query_input_active {
                app.set_status("Loading graph...");
                spawn_graph_query(app, tx);
            }
        }

        // Multi-server select (alerts and query only)
        KeyCode::Char('m') => {
            if (app.active_resource == "alerts" || app.active_resource == "query")
                && !app.query_input_active
                && app.tprom_config.servers.len() > 1
            {
                app.multi_server_cursor = 0;
                app.mode = Mode::MultiServerSelect;
            }
        }

        // Alert exclusion dialog
        KeyCode::Char('x') => {
            if app.active_resource == "alerts" || app.active_resource == "alertexclusion" {
                let server_names: Vec<String> = app
                    .tprom_config
                    .servers
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                app.exclusion_dialog_state =
                    ui::exclusion_dialog::ExclusionDialogState::new(&server_names);
                app.mode = Mode::ExclusionDialog;
            }
        }

        // Right arrow: expand collapsed group on header row
        KeyCode::Right | KeyCode::Char('l') => {
            if app.multi_server_mode_active && app.active_resource == "alerts" {
                if let Some(server) =
                    app.multi_server_alert_server_at(app.multi_server_flat_alert_index)
                {
                    if app.multi_server_collapsed_alerts.contains(&server) {
                        app.multi_server_collapsed_alerts.remove(&server);
                    }
                }
            } else if app.multi_server_mode_active
                && app.active_resource == "query"
                && !app.query_input_active
            {
                if let Some(server) =
                    app.multi_server_query_server_at(app.multi_server_flat_query_index)
                {
                    if app.multi_server_collapsed_query.contains(&server) {
                        app.multi_server_collapsed_query.remove(&server);
                    }
                }
            }
        }

        // Left arrow: collapse expanded group on header row
        KeyCode::Left | KeyCode::Char('h') => {
            if app.multi_server_mode_active && app.active_resource == "alerts" {
                if let Some(server) =
                    app.multi_server_alert_server_at(app.multi_server_flat_alert_index)
                {
                    app.multi_server_collapsed_alerts.insert(server);
                }
            } else if app.multi_server_mode_active
                && app.active_resource == "query"
                && !app.query_input_active
            {
                if let Some(server) =
                    app.multi_server_query_server_at(app.multi_server_flat_query_index)
                {
                    app.multi_server_collapsed_query.insert(server);
                }
            }
        }

        // Expand all groups
        KeyCode::Char('E') => {
            if app.multi_server_mode_active && app.active_resource == "alerts" {
                app.multi_server_collapsed_alerts.clear();
                app.multi_server_flat_alert_index = 0;
                app.multi_alert_table_state.select(Some(0));
            } else if app.multi_server_mode_active
                && app.active_resource == "query"
                && !app.query_input_active
            {
                app.multi_server_collapsed_query.clear();
                app.multi_server_flat_query_index = 0;
            }
        }

        // Collapse all groups
        KeyCode::Char('W') => {
            if app.multi_server_mode_active && app.active_resource == "alerts" {
                let filtered = app.filtered_multi_server_alerts();
                app.multi_server_collapsed_alerts =
                    filtered.iter().map(|(name, _)| name.clone()).collect();
                app.multi_server_flat_alert_index = 0;
                app.multi_alert_table_state.select(Some(0));
            } else if app.multi_server_mode_active
                && app.active_resource == "query"
                && !app.query_input_active
            {
                let filtered = app.filtered_multi_server_query();
                app.multi_server_collapsed_query =
                    filtered.iter().map(|(name, _)| name.clone()).collect();
                app.multi_server_flat_query_index = 0;
            }
        }

        // Refresh
        KeyCode::Char('R') => {
            app.set_status("Refreshing...");
            if app.multi_server_mode_active && app.active_resource == "alerts" {
                spawn_multi_server_alerts(app, tx);
            } else if app.multi_server_mode_active && app.active_resource == "query" {
                spawn_multi_server_query(app, tx);
            } else {
                spawn_fetch(app, &app.active_resource.clone(), tx);
            }
        }

        // Tab: switch panels (labels, query)
        KeyCode::Tab => {
            if app.active_resource == "labels" {
                app.label_focus_right = !app.label_focus_right;
            } else if app.active_resource == "query" {
                app.query_input_active = !app.query_input_active;
            }
        }

        // Servers: connect
        KeyCode::Char('c') => {
            if app.active_resource == "servers" && !app.tprom_config.servers.is_empty() {
                let server = app.tprom_config.servers[app.selected_server_index].clone();
                app.current_server = Some(server);
                app.mode = Mode::Splash;
                app.splash_state = ui::splash::SplashState::new();

                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let _ = tx_clone
                        .send(AppEvent::Progress(
                            "Connecting to Prometheus...".to_string(),
                        ))
                        .await;
                    let _ = tx_clone.send(AppEvent::Connect).await;
                });
            }
        }

        // Toggle sound for new firing alerts
        KeyCode::Char('b') => {
            app.tprom_config.alert_sound = !app.tprom_config.alert_sound;
            let state = if app.tprom_config.alert_sound {
                "ON"
            } else {
                "OFF"
            };
            app.save_config_with_status(format!("Alert sound {}", state));
        }

        // Auto-refresh interval
        KeyCode::Char('+') | KeyCode::Char('=') => app.step_refresh_interval(true),
        KeyCode::Char('-') => app.step_refresh_interval(false),

        // Servers: add / Auto-refresh toggle
        KeyCode::Char('a') => {
            if app.active_resource == "servers" {
                app.mode = Mode::ServerDialog;
                app.server_dialog_state.clear();
            } else {
                // Toggle auto-refresh
                app.auto_refresh_enabled = !app.auto_refresh_enabled;
                app.tprom_config.auto_refresh = app.auto_refresh_enabled;
                if app.auto_refresh_enabled {
                    app.last_auto_refresh = Some(Instant::now());
                    app.save_config_with_status(format!(
                        "Auto-refresh ON ({}s, +/- to change)",
                        app.auto_refresh_interval.as_secs()
                    ));
                } else {
                    app.last_auto_refresh = None;
                    app.save_config_with_status("Auto-refresh OFF".to_string());
                }
            }
        }

        // Servers: delete / Exclusion: delete
        KeyCode::Char('D') => {
            if app.active_resource == "servers" && !app.tprom_config.servers.is_empty() {
                let server = &app.tprom_config.servers[app.selected_server_index];
                app.pending_action = Some(PendingAction {
                    key: server.name.clone(),
                    action_type: PendingActionType::DeleteServer,
                    selected_yes: false,
                });
                app.mode = Mode::Confirm;
            } else if app.active_resource == "alertexclusion"
                && !app.tprom_config.alert_exclusions.is_empty()
            {
                let exc = &app.tprom_config.alert_exclusions[app.selected_exclusion_index];
                app.pending_action = Some(PendingAction {
                    key: exc.pattern.clone(),
                    action_type: PendingActionType::DeleteExclusion,
                    selected_yes: false,
                });
                app.mode = Mode::Confirm;
            }
        }

        KeyCode::Esc => {
            // Clear filter if active, or clear search
            if !app.filter_text.is_empty() {
                app.filter_text.clear();
                if app.active_resource.as_str() == "metrics" {
                    app.apply_metric_filter()
                }
                // Restore collapsed state and reset flat index after clearing filter
                if app.multi_server_mode_active && app.active_resource == "alerts" {
                    if let Some(saved) = app.pre_filter_collapsed_alerts.take() {
                        app.multi_server_collapsed_alerts = saved;
                    }
                    app.multi_server_flat_alert_index = 0;
                } else if app.multi_server_mode_active && app.active_resource == "query" {
                    if let Some(saved) = app.pre_filter_collapsed_query.take() {
                        app.multi_server_collapsed_query = saved;
                    }
                    app.multi_server_flat_query_index = 0;
                }
            }
            if !app.config_search_text.is_empty() {
                app.config_search_text.clear();
                app.config_search_matches.clear();
                app.config_search_current = 0;
            }
        }

        // Help cheat sheet
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }

        _ => {}
    }

    // After navigation in labels left panel, fetch values
    if app.active_resource == "labels"
        && !app.label_focus_right
        && matches!(
            key,
            KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Down | KeyCode::Up
        )
    {
        spawn_label_values(app, tx);
    }
}

async fn handle_query_input_keys(app: &mut App, key: KeyCode, tx: &mpsc::Sender<AppEvent>) {
    match key {
        KeyCode::Esc => {
            if app.autocomplete_visible {
                // Close autocomplete first
                app.autocomplete_visible = false;
                app.autocomplete_suggestions.clear();
            } else if !app.query_results.is_empty() {
                // Switch focus to results
                app.query_input_active = false;
            }
        }
        KeyCode::Tab => {
            if app.autocomplete_visible {
                app.accept_autocomplete();
            } else if !app.query_results.is_empty() {
                app.query_input_active = false;
            }
        }
        KeyCode::Enter => {
            app.autocomplete_visible = false;
            app.autocomplete_suggestions.clear();
            if app.multi_server_mode_active {
                spawn_multi_server_query(app, tx);
            } else {
                spawn_fetch(app, "query", tx);
            }
        }
        KeyCode::Up => {
            if app.autocomplete_visible {
                if app.autocomplete_selected == 0 {
                    app.autocomplete_selected =
                        app.autocomplete_suggestions.len().saturating_sub(1);
                } else {
                    app.autocomplete_selected -= 1;
                }
            } else {
                app.query_history_prev();
                app.update_autocomplete();
            }
        }
        KeyCode::Down => {
            if app.autocomplete_visible {
                app.autocomplete_selected =
                    (app.autocomplete_selected + 1) % app.autocomplete_suggestions.len().max(1);
            } else {
                app.query_history_next();
                app.update_autocomplete();
            }
        }
        KeyCode::Backspace => {
            app.query_input.pop();
            app.query_cursor_pos = app.query_input.len();
            // Fetch label values if needed, then update autocomplete
            if let Some(label) = app.pending_label_values_fetch() {
                let _ = app.fetch_cached_label_values(&label).await;
            }
            app.update_autocomplete();
        }
        KeyCode::Char(c) => {
            app.query_input.push(c);
            app.query_cursor_pos = app.query_input.len();
            // Fetch label values if needed, then update autocomplete
            if let Some(label) = app.pending_label_values_fetch() {
                let _ = app.fetch_cached_label_values(&label).await;
            }
            app.update_autocomplete();
        }
        _ => {}
    }
}

/// Spawn a background task to fetch data for the given resource.
/// Sets app.loading = true and sends results back via the channel.
fn spawn_fetch(app: &mut App, resource: &str, tx: &mpsc::Sender<AppEvent>) {
    let client = match app.client.clone() {
        Some(c) => c,
        None => return,
    };
    let resource = resource.to_string();

    match resource.as_str() {
        "servers" | "alertexclusion" => return, // local config, no fetch needed
        "query" => {
            let expr = app.query_input.trim().to_string();
            if expr.is_empty() {
                app.query_input_active = true;
                return;
            }
            // Add to history (avoid duplicates at the end)
            if app.query_history.last().map(|s| s.as_str()) != Some(&expr) {
                app.query_history.push(expr.clone());
            }
            app.query_history_index = None;
            app.loading = true;
            let tx_bg = tx.clone();
            tokio::spawn(async move {
                match client.query_instant(&expr).await {
                    Ok(data) => {
                        let _ = tx_bg
                            .send(AppEvent::FetchDone(FetchResult::Query {
                                result_type: data.result_type,
                                results: data.result,
                            }))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx_bg
                            .send(AppEvent::FetchDone(FetchResult::ResourceError {
                                resource: "query".to_string(),
                                error: format!("{}", e),
                            }))
                            .await;
                    }
                }
            });
            return;
        }
        _ => {}
    }

    app.loading = true;
    let tx_bg = tx.clone();
    tokio::spawn(async move {
        let result = match resource.as_str() {
            "targets" => match client.fetch_targets().await {
                Ok(data) => FetchResult::Targets(data.active_targets),
                Err(e) => FetchResult::ResourceError {
                    resource,
                    error: format!("{}", e),
                },
            },
            "alerts" => match client.fetch_alerts().await {
                Ok(data) => FetchResult::Alerts(data.alerts),
                Err(e) => FetchResult::ResourceError {
                    resource,
                    error: format!("{}", e),
                },
            },
            "rules" => match client.fetch_rules().await {
                Ok(data) => {
                    let mut flat_rules = Vec::new();
                    for group in &data.groups {
                        for rule in &group.rules {
                            flat_rules.push((group.name.clone(), rule.clone()));
                        }
                    }
                    FetchResult::Rules {
                        groups: data.groups,
                        flat_rules,
                    }
                }
                Err(e) => FetchResult::ResourceError {
                    resource,
                    error: format!("{}", e),
                },
            },
            "metrics" => {
                let names = client.fetch_metric_names().await.unwrap_or_default();
                let metadata = client.fetch_metadata().await.unwrap_or_default();
                FetchResult::Metrics { names, metadata }
            }
            "labels" => match client.fetch_label_names().await {
                Ok(names) => {
                    let values = if !names.is_empty() {
                        client
                            .fetch_label_values(&names[0])
                            .await
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    FetchResult::Labels { names, values }
                }
                Err(e) => FetchResult::ResourceError {
                    resource,
                    error: format!("{}", e),
                },
            },
            "tsdb" => match client.fetch_tsdb_status().await {
                Ok(data) => FetchResult::TsdbStatus(data),
                Err(e) => FetchResult::ResourceError {
                    resource,
                    error: format!("{}", e),
                },
            },
            "config" => match client.fetch_config().await {
                Ok(data) => FetchResult::Config(data.yaml),
                Err(e) => FetchResult::ResourceError {
                    resource,
                    error: format!("{}", e),
                },
            },
            "info" => {
                let build_info = client.fetch_build_info().await.ok();
                let runtime_info = client.fetch_runtime_info().await.ok();
                let flags_map = client.fetch_flags().await.unwrap_or_default();
                let mut flags: Vec<(String, String)> = flags_map.into_iter().collect();
                flags.sort_by(|a, b| a.0.cmp(&b.0));
                FetchResult::Info {
                    build_info,
                    runtime_info,
                    flags,
                }
            }
            _ => return,
        };
        let _ = tx_bg.send(AppEvent::FetchDone(result)).await;
    });
}

/// Spawn a background task for graph query
fn spawn_graph_query(app: &mut App, tx: &mpsc::Sender<AppEvent>) {
    let expr = app.query_input.trim().to_string();
    if expr.is_empty() {
        return;
    }
    let client = match app.client.clone() {
        Some(c) => c,
        None => return,
    };
    app.loading = true;
    let tx_bg = tx.clone();
    tokio::spawn(async move {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let start = now - 3600.0;
        let end = now;
        let step = (end - start) / 240.0;

        match client.query_range(&expr, start, end, step).await {
            Ok(data) => {
                let mut graph_data = Vec::new();
                let mut value_min = f64::MAX;
                let mut value_max = f64::MIN;
                for result in &data.result {
                    let label = result.label_string();
                    let mut points: Vec<(f64, f64)> = Vec::new();
                    if let Some(ref values) = result.values {
                        for (ts, val_str) in values {
                            if let Ok(val) = val_str.parse::<f64>() {
                                if val.is_finite() {
                                    if val < value_min {
                                        value_min = val;
                                    }
                                    if val > value_max {
                                        value_max = val;
                                    }
                                    points.push((*ts, val));
                                }
                            }
                        }
                    }
                    if !points.is_empty() {
                        graph_data.push((label, points));
                    }
                }
                if value_min == value_max {
                    value_min -= 1.0;
                    value_max += 1.0;
                } else {
                    let range = value_max - value_min;
                    value_min -= range * 0.05;
                    value_max += range * 0.05;
                }
                let _ = tx_bg
                    .send(AppEvent::FetchDone(FetchResult::GraphQuery {
                        graph_data,
                        time_min: start,
                        time_max: end,
                        value_min,
                        value_max,
                    }))
                    .await;
            }
            Err(e) => {
                let _ = tx_bg
                    .send(AppEvent::FetchDone(FetchResult::ResourceError {
                        resource: "graph".to_string(),
                        error: format!("{}", e),
                    }))
                    .await;
            }
        }
    });
}

/// Spawn a background task for describe metric (the only describe that does HTTP)
fn spawn_describe_metric(app: &mut App, tx: &mpsc::Sender<AppEvent>) {
    if app.filtered_metrics.is_empty() || app.selected_metric_index >= app.filtered_metrics.len() {
        return;
    }
    let name = app.filtered_metrics[app.selected_metric_index].clone();
    let info = if let Some(metas) = app.metadata.get(&name) {
        if let Some(m) = metas.first() {
            MetricInfo {
                name: name.clone(),
                metric_type: m.metric_type.clone(),
                help: m.help.clone(),
            }
        } else {
            MetricInfo {
                name: name.clone(),
                metric_type: "unknown".to_string(),
                help: String::new(),
            }
        }
    } else {
        MetricInfo {
            name: name.clone(),
            metric_type: "unknown".to_string(),
            help: String::new(),
        }
    };

    let client = match app.client.clone() {
        Some(c) => c,
        None => return,
    };
    app.loading = true;
    let tx_bg = tx.clone();
    tokio::spawn(async move {
        let results = client
            .query_instant(&name)
            .await
            .map(|d| d.result)
            .unwrap_or_default();
        let _ = tx_bg
            .send(AppEvent::FetchDone(FetchResult::DescribeMetric {
                data: DescribeData::Metric {
                    name,
                    info,
                    results,
                },
            }))
            .await;
    });
}

/// Spawn a background task for fetching label values
fn spawn_label_values(app: &mut App, tx: &mpsc::Sender<AppEvent>) {
    let client = match app.client.clone() {
        Some(c) => c,
        None => return,
    };
    if app.selected_label_index >= app.label_names.len() {
        return;
    }
    let label = app.label_names[app.selected_label_index].clone();
    let tx_bg = tx.clone();
    tokio::spawn(async move {
        let values = client.fetch_label_values(&label).await.unwrap_or_default();
        let _ = tx_bg
            .send(AppEvent::FetchDone(FetchResult::LabelValues { values }))
            .await;
    });
}

/// Spawn parallel alert fetches across all selected servers
fn spawn_multi_server_alerts(app: &mut App, tx: &mpsc::Sender<AppEvent>) {
    let selected_names = app.multi_server_selected.clone();
    let servers: Vec<ServerConfig> = app
        .tprom_config
        .servers
        .iter()
        .filter(|s| selected_names.contains(&s.name))
        .cloned()
        .collect();

    if servers.is_empty() {
        return;
    }

    app.loading = true;
    let tx_bg = tx.clone();

    tokio::spawn(async move {
        let mut handles = Vec::new();
        for server in servers {
            let name = server.name.clone();
            handles.push(tokio::spawn(async move {
                let client = match client::PrometheusClient::new(&server.url, server.auth.clone()) {
                    Ok(c) => c,
                    Err(_) => return (name, None),
                };
                let alerts = client.fetch_alerts().await.map(|d| d.alerts).ok();
                (name, alerts)
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        let _ = tx_bg
            .send(AppEvent::FetchDone(FetchResult::MultiServerAlerts(results)))
            .await;
    });
}

/// Spawn parallel query execution across all selected servers
fn spawn_multi_server_query(app: &mut App, tx: &mpsc::Sender<AppEvent>) {
    let expr = app.query_input.trim().to_string();
    if expr.is_empty() {
        return;
    }

    let selected_names = app.multi_server_selected.clone();
    let servers: Vec<ServerConfig> = app
        .tprom_config
        .servers
        .iter()
        .filter(|s| selected_names.contains(&s.name))
        .cloned()
        .collect();

    if servers.is_empty() {
        return;
    }

    // Add to history
    if app.query_history.last().map(|s| s.as_str()) != Some(&expr) {
        app.query_history.push(expr.clone());
    }
    app.query_history_index = None;

    app.loading = true;
    let tx_bg = tx.clone();

    tokio::spawn(async move {
        let mut handles = Vec::new();
        let mut first_result_type = String::new();

        for server in servers {
            let name = server.name.clone();
            let expr = expr.clone();
            handles.push(tokio::spawn(async move {
                let client = match client::PrometheusClient::new(&server.url, server.auth.clone()) {
                    Ok(c) => c,
                    Err(_) => return (name, String::new(), Vec::new()),
                };
                match client.query_instant(&expr).await {
                    Ok(data) => (name, data.result_type, data.result),
                    Err(_) => (name, String::new(), Vec::new()),
                }
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok((name, result_type, data)) = handle.await {
                if first_result_type.is_empty() && !result_type.is_empty() {
                    first_result_type = result_type;
                }
                results.push((name, data));
            }
        }

        let _ = tx_bg
            .send(AppEvent::FetchDone(FetchResult::MultiServerQuery {
                results,
                result_type: first_result_type,
            }))
            .await;
    });
}
