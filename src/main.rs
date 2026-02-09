#![allow(dead_code)]

mod app;
mod client;
mod model;
mod ui;

use anyhow::Result;
use app::{App, Mode, PendingAction, PendingActionType};
use clap::Parser;
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
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
enum AppEvent {
    Progress(String),
    Connect,
    Connected,
    Error(String),
}

/// TProm - Terminal UI for Prometheus
#[derive(Parser, Debug)]
#[command(name = "tprom", version = VERSION, about = "Terminal UI for Prometheus")]
pub struct Args {
    /// Prometheus server URL
    #[arg(short, long)]
    pub url: Option<String>,
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

    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);

    // Determine initial mode
    if let Some(ref url) = args.url {
        // CLI URL provided, use directly
        app.current_server = Some(model::ServerConfig {
            name: url.clone(),
            url: url.clone(),
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = tx_clone
                .send(AppEvent::Progress("Connecting to Prometheus...".to_string()))
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
                .send(AppEvent::Progress("Connecting to Prometheus...".to_string()))
                .await;
            let _ = tx_clone.send(AppEvent::Connect).await;
        });
    }

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        // Process async events
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Progress(msg) => {
                    app.splash_state.message = msg;
                }
                AppEvent::Connect => {
                    if let Err(e) = app.connect() {
                        app.status_message = Some(format!("Connection failed: {}", e));
                        app.mode = Mode::ServerDialog;
                    } else {
                        // Test connection with build info
                        if let Some(ref client) = app.client {
                            match client.health_check().await {
                                Ok(bi) => {
                                    app.build_info = Some(bi);
                                    app.mode = Mode::Normal;
                                    // Fetch initial data for default resource
                                    let _ = app.fetch_targets().await;
                                    app.status_message =
                                        Some("Connected successfully".to_string());
                                }
                                Err(e) => {
                                    app.status_message =
                                        Some(format!("Connection failed: {}", e));
                                    app.mode = Mode::ServerDialog;
                                    app.client = None;
                                }
                            }
                        }
                    }
                }
                AppEvent::Connected => {
                    app.mode = Mode::Normal;
                }
                AppEvent::Error(msg) => {
                    app.status_message = Some(msg);
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                // Global quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
                {
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
                        handle_resources_keys(&mut app, key.code).await;
                    }
                    Mode::Confirm => {
                        handle_confirm_keys(&mut app, key.code);
                    }
                    Mode::Describe => {
                        match key.code {
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
                        }
                    }
                    Mode::Normal => {
                        if app.filter_active {
                            handle_filter_keys(&mut app, key.code).await;
                        } else if app.config_search_active {
                            handle_config_search_keys(&mut app, key.code);
                        } else {
                            handle_normal_keys(&mut app, key.code, &tx).await;
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
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
            app.server_dialog_state.active_field = match app.server_dialog_state.active_field {
                DialogField::Name => DialogField::Url,
                DialogField::Url => DialogField::Name,
            };
        }
        KeyCode::Enter => {
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
                        .send(AppEvent::Progress("Connecting to Prometheus...".to_string()))
                        .await;
                    let _ = tx_clone.send(AppEvent::Connect).await;
                });
            }
        }
        KeyCode::Backspace => match app.server_dialog_state.active_field {
            DialogField::Name => {
                app.server_dialog_state.name.pop();
            }
            DialogField::Url => {
                app.server_dialog_state.url.pop();
            }
        },
        KeyCode::Char(c) => match app.server_dialog_state.active_field {
            DialogField::Name => {
                app.server_dialog_state.name.push(c);
            }
            DialogField::Url => {
                app.server_dialog_state.url.push(c);
            }
        },
        _ => {}
    }
}

async fn handle_resources_keys(app: &mut App, key: KeyCode) {
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

                // Fetch data for the selected resource
                let _ = fetch_resource_data(app, &cmd).await;
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
                    }
                }
            }
            app.pending_action = None;
            app.mode = Mode::Normal;
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
            match app.active_resource.as_str() {
                "metrics" => app.apply_metric_filter(),
                _ => {}
            }
        }
        KeyCode::Enter => {
            app.filter_active = false;
            // Keep filter applied
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            match app.active_resource.as_str() {
                "metrics" => app.apply_metric_filter(),
                _ => {}
            }
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            match app.active_resource.as_str() {
                "metrics" => app.apply_metric_filter(),
                _ => {}
            }
        }
        _ => {}
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
    match key {
        KeyCode::Char('q') => app.should_quit = true,
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
                }
                "config" => {
                    app.config_search_active = true;
                    app.config_search_text.clear();
                    app.config_search_matches.clear();
                    app.config_search_current = 0;
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
        KeyCode::Char('d') | KeyCode::Enter => {
            match app.active_resource.as_str() {
                "targets" => {
                    let _ = app.describe_target().await;
                    if !matches!(app.describe_data, app::DescribeData::None) {
                        app.mode = Mode::Describe;
                    }
                }
                "alerts" => {
                    let _ = app.describe_alert().await;
                    if !matches!(app.describe_data, app::DescribeData::None) {
                        app.mode = Mode::Describe;
                    }
                }
                "rules" => {
                    let _ = app.describe_rule().await;
                    if !matches!(app.describe_data, app::DescribeData::None) {
                        app.mode = Mode::Describe;
                    }
                }
                "metrics" => {
                    let _ = app.describe_metric().await;
                    if !matches!(app.describe_data, app::DescribeData::None) {
                        app.mode = Mode::Describe;
                    }
                }
                _ => {}
            }
        }

        // Refresh
        KeyCode::Char('R') => {
            app.status_message = Some("Refreshing...".to_string());
            let result = fetch_resource_data(app, &app.active_resource.clone()).await;
            if let Err(e) = result {
                app.status_message = Some(format!("Error: {}", e));
            } else {
                app.status_message = Some("Refreshed".to_string());
            }
        }

        // Labels: Tab to switch panels
        KeyCode::Tab => {
            if app.active_resource == "labels" {
                app.label_focus_right = !app.label_focus_right;
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
                        .send(AppEvent::Progress("Connecting to Prometheus...".to_string()))
                        .await;
                    let _ = tx_clone.send(AppEvent::Connect).await;
                });
            }
        }

        // Servers: add
        KeyCode::Char('a') => {
            if app.active_resource == "servers" {
                app.mode = Mode::ServerDialog;
                app.server_dialog_state.clear();
            }
        }

        // Servers: delete (Ctrl-d)
        KeyCode::Char('D') => {
            if app.active_resource == "servers" && !app.tprom_config.servers.is_empty() {
                let server = &app.tprom_config.servers[app.selected_server_index];
                app.pending_action = Some(PendingAction {
                    key: server.name.clone(),
                    action_type: PendingActionType::DeleteServer,
                    selected_yes: false,
                });
                app.mode = Mode::Confirm;
            }
        }

        KeyCode::Esc => {
            // Clear filter if active, or clear search
            if !app.filter_text.is_empty() {
                app.filter_text.clear();
                match app.active_resource.as_str() {
                    "metrics" => app.apply_metric_filter(),
                    _ => {}
                }
            }
            if !app.config_search_text.is_empty() {
                app.config_search_text.clear();
                app.config_search_matches.clear();
                app.config_search_current = 0;
            }
        }

        _ => {}
    }

    // After navigation in labels left panel, fetch values
    if app.active_resource == "labels"
        && !app.label_focus_right
        && matches!(key, KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Down | KeyCode::Up)
    {
        let _ = app.fetch_label_values_for_current().await;
    }
}

async fn fetch_resource_data(app: &mut App, resource: &str) -> Result<()> {
    match resource {
        "targets" => app.fetch_targets().await,
        "alerts" => app.fetch_alerts().await,
        "rules" => app.fetch_rules().await,
        "metrics" => app.fetch_metrics().await,
        "labels" => app.fetch_labels().await,
        "tsdb" => app.fetch_tsdb_status().await,
        "config" => app.fetch_config().await,
        "info" => app.fetch_info().await,
        "servers" => Ok(()), // servers are local config, no fetch needed
        _ => Ok(()),
    }
}
