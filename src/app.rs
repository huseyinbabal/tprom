use crate::client::PrometheusClient;
use crate::model::*;
use crate::ui::exclusion_dialog::ExclusionDialogState;
use crate::ui::server_dialog::ServerDialogState;
use crate::ui::splash::SplashState;
use anyhow::Result;
use ratatui::widgets::TableState;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mode {
    Splash,
    Normal,
    Describe,
    Confirm,
    Resources,
    ServerDialog,
    MultiServerSelect,
    ExclusionDialog,
    Help,
}

#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub name: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingActionType {
    DeleteServer,
    DeleteExclusion,
}

pub struct PendingAction {
    pub key: String,
    pub action_type: PendingActionType,
    pub selected_yes: bool,
}

/// Holds all describe-view content variants
#[derive(Debug, Clone)]
pub enum DescribeData {
    Target(ActiveTarget),
    Alert(Alert),
    Rule(Rule),
    Metric {
        name: String,
        info: MetricInfo,
        results: Vec<QueryResult>,
    },
    QueryResult(QueryResult),
    None,
}

#[derive(Debug, Clone)]
pub struct AutocompleteItem {
    pub text: String,
    pub kind: AutocompleteKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AutocompleteKind {
    Metric,
    Function,
    Keyword,
    LabelName,
    Operator,
    LabelValue,
}

impl AutocompleteKind {
    pub fn label(&self) -> &'static str {
        match self {
            AutocompleteKind::Metric => "metric",
            AutocompleteKind::Function => "fn",
            AutocompleteKind::Keyword => "kw",
            AutocompleteKind::LabelName => "label",
            AutocompleteKind::Operator => "op",
            AutocompleteKind::LabelValue => "value",
        }
    }
}

/// Context within `{}` label matchers for autocomplete
#[derive(Debug, Clone, PartialEq)]
pub enum LabelContext {
    /// Not inside braces — normal metric/function/keyword completion
    None,
    /// After `{` or `,` — expecting a label name
    LabelName { prefix: String },
    /// After a label name — expecting an operator
    Operator { label_name: String },
    /// After an operator — expecting a value (possibly with partial quote)
    LabelValue { label_name: String, prefix: String },
}

pub struct App {
    pub mode: Mode,
    pub active_resource: String,
    pub splash_state: SplashState,

    // Server configuration
    pub tprom_config: TpromConfig,
    pub current_server: Option<ServerConfig>,
    pub server_dialog_state: ServerDialogState,
    pub selected_server_index: usize,

    // Prometheus client
    pub client: Option<PrometheusClient>,

    // Data - Targets
    pub targets: Vec<ActiveTarget>,
    pub selected_target_index: usize,

    // Data - Alerts
    pub alerts: Vec<Alert>,
    pub selected_alert_index: usize,
    pub alert_table_state: TableState,
    pub multi_alert_table_state: TableState,

    // Data - Rules
    pub rule_groups: Vec<RuleGroup>,
    pub flat_rules: Vec<(String, Rule)>, // (group_name, rule)
    pub selected_rule_index: usize,

    // Data - Metrics
    pub metric_names: Vec<String>,
    pub filtered_metrics: Vec<String>,
    pub metadata: HashMap<String, Vec<MetricMetadata>>,
    pub selected_metric_index: usize,

    // Data - Labels
    pub label_names: Vec<String>,
    pub label_values: Vec<String>,
    pub selected_label_index: usize,
    pub selected_label_value_index: usize,
    pub label_focus_right: bool, // true = focus on values panel

    // Data - TSDB Status
    pub tsdb_status: Option<TsdbStatusData>,
    pub tsdb_scroll: usize,

    // Data - Config
    pub config_yaml: String,
    pub config_scroll: usize,

    // Data - Info (runtime + build + flags)
    pub build_info: Option<BuildInfoData>,
    pub runtime_info: Option<RuntimeInfoData>,
    pub flags: Vec<(String, String)>,
    pub info_scroll: usize,

    // Search / filter (for "/" input)
    pub filter_text: String,
    pub filter_active: bool,

    // Config search
    pub config_search_active: bool,
    pub config_search_text: String,
    pub config_search_matches: Vec<usize>,
    pub config_search_current: usize,

    // Info search
    pub info_search_active: bool,
    pub info_search_text: String,

    pub should_quit: bool,

    // Resources modal
    pub resources: Vec<ResourceItem>,
    pub command_text: String,
    pub command_suggestions: Vec<ResourceItem>,
    pub command_suggestion_selected: usize,
    pub command_preview: Option<String>,

    // Describe data
    pub describe_data: DescribeData,
    pub describe_scroll: usize,

    // Confirm action
    pub pending_action: Option<PendingAction>,
    pub last_key_press: Option<(crossterm::event::KeyCode, std::time::Instant)>,

    // Auto-refresh
    pub auto_refresh_enabled: bool,
    pub auto_refresh_interval: std::time::Duration,
    pub last_auto_refresh: Option<std::time::Instant>,

    // Data - Query (PromQL REPL)
    pub query_input: String,
    pub query_results: Vec<QueryResult>,
    pub query_result_type: String,
    pub selected_query_result_index: usize,
    pub query_input_active: bool,
    pub query_history: Vec<String>,
    pub query_history_index: Option<usize>,
    pub query_cursor_pos: usize,

    // Autocomplete
    pub autocomplete_suggestions: Vec<AutocompleteItem>,
    pub autocomplete_selected: usize,
    pub autocomplete_visible: bool,
    pub autocomplete_pool: Vec<AutocompleteItem>, // all completable items

    // Label autocomplete caches (for inside-braces completion)
    pub cached_label_names: Vec<String>,
    pub cached_label_values: HashMap<String, Vec<String>>,

    // Graph view
    pub graph_visible: bool,
    /// Each series: (label_string, Vec<(timestamp, value)>)
    pub graph_data: Vec<(String, Vec<(f64, f64)>)>,
    pub graph_time_min: f64,
    pub graph_time_max: f64,
    pub graph_value_min: f64,
    pub graph_value_max: f64,
    /// Currently hovered legend item index (None = no highlight)
    pub graph_legend_hovered: Option<usize>,

    // Status bar message
    pub status_message: Option<String>,
    pub status_is_error: bool,
    pub status_set_at: Option<std::time::Instant>,

    // Loading state
    pub loading: bool,
    pub spinner_frame: usize,

    // Multi-server selection
    pub multi_server_selected: Vec<String>, // persisted selected server names
    pub multi_server_cursor: usize,         // cursor in the modal list
    pub multi_server_mode_active: bool,     // true = multi-server results are displayed

    // Multi-server results (tree view data)
    pub multi_server_alerts: Vec<(String, Vec<Alert>)>, // (server_name, alerts)
    pub multi_server_query_results: Vec<(String, Vec<QueryResult>)>, // (server_name, results)
    pub multi_server_query_result_type: String,
    pub multi_server_flat_alert_index: usize, // navigation index across flattened tree

    // New-alert notifications
    /// Per server: names of notifiable alerts firing at its last successful fetch.
    /// A server missing here has no baseline yet, so its first fetch stays silent.
    pub firing_seen: HashMap<String, HashSet<String>>,
    /// Last time each (server, alertname) was seen firing, to ignore flapping
    pub firing_last_seen: HashMap<(String, String), std::time::Instant>,
    /// Notifications shown until the user dismisses them (newest first)
    pub alert_toasts: Vec<AlertToast>,
    /// `--notification-sound`: replaces the configured sound for this session
    /// only, so it is never written back to the config file
    pub notification_sound_override: Option<String>,
    pub multi_server_flat_query_index: usize,
    pub multi_server_collapsed_alerts: HashSet<String>, // collapsed server names (alerts)
    pub multi_server_collapsed_query: HashSet<String>,  // collapsed server names (query)
    pub pre_filter_collapsed_alerts: Option<HashSet<String>>, // saved state before filter
    pub pre_filter_collapsed_query: Option<HashSet<String>>, // saved state before filter

    // Alert exclusion dialog
    pub exclusion_dialog_state: ExclusionDialogState,
    /// Compiled regexes from config, rebuilt when exclusions change
    pub compiled_exclusions: Vec<(Regex, Vec<String>)>, // (pattern, server_names)

    // Alert exclusions management view
    pub selected_exclusion_index: usize,
}

impl App {
    pub fn new() -> Self {
        let resources = vec![
            ResourceItem {
                name: "Servers".to_string(),
                command: "servers".to_string(),
                description: "Manage server connections".to_string(),
            },
            ResourceItem {
                name: "Targets".to_string(),
                command: "targets".to_string(),
                description: "Scrape targets and health".to_string(),
            },
            ResourceItem {
                name: "Alerts".to_string(),
                command: "alerts".to_string(),
                description: "Firing and pending alerts".to_string(),
            },
            ResourceItem {
                name: "Alert Exclusions".to_string(),
                command: "alertexclusion".to_string(),
                description: "Manage alert exclusion rules".to_string(),
            },
            ResourceItem {
                name: "Rules".to_string(),
                command: "rules".to_string(),
                description: "Recording and alerting rules".to_string(),
            },
            ResourceItem {
                name: "Metrics".to_string(),
                command: "metrics".to_string(),
                description: "Browse metric names".to_string(),
            },
            ResourceItem {
                name: "Labels".to_string(),
                command: "labels".to_string(),
                description: "Browse label names and values".to_string(),
            },
            ResourceItem {
                name: "TSDB".to_string(),
                command: "tsdb".to_string(),
                description: "TSDB status and cardinality".to_string(),
            },
            ResourceItem {
                name: "Config".to_string(),
                command: "config".to_string(),
                description: "Prometheus configuration".to_string(),
            },
            ResourceItem {
                name: "Info".to_string(),
                command: "info".to_string(),
                description: "Runtime, build info and flags".to_string(),
            },
            ResourceItem {
                name: "Query".to_string(),
                command: "query".to_string(),
                description: "Interactive PromQL query".to_string(),
            },
        ];

        let tprom_config = TpromConfig::load();
        let auto_refresh_enabled = tprom_config.auto_refresh;
        let auto_refresh_interval =
            std::time::Duration::from_secs(tprom_config.refresh_interval_secs.max(1));

        Self {
            mode: Mode::Splash,
            active_resource: "alerts".to_string(),
            splash_state: SplashState::new(),
            tprom_config,
            current_server: None,
            server_dialog_state: ServerDialogState::new(),
            selected_server_index: 0,
            client: None,
            targets: Vec::new(),
            selected_target_index: 0,
            alerts: Vec::new(),
            selected_alert_index: 0,
            alert_table_state: TableState::default().with_selected(0),
            multi_alert_table_state: TableState::default().with_selected(0),
            rule_groups: Vec::new(),
            flat_rules: Vec::new(),
            selected_rule_index: 0,
            metric_names: Vec::new(),
            filtered_metrics: Vec::new(),
            metadata: HashMap::new(),
            selected_metric_index: 0,
            label_names: Vec::new(),
            label_values: Vec::new(),
            selected_label_index: 0,
            selected_label_value_index: 0,
            label_focus_right: false,
            tsdb_status: None,
            tsdb_scroll: 0,
            config_yaml: String::new(),
            config_scroll: 0,
            build_info: None,
            runtime_info: None,
            flags: Vec::new(),
            info_scroll: 0,
            filter_text: String::new(),
            filter_active: false,
            config_search_active: false,
            config_search_text: String::new(),
            config_search_matches: Vec::new(),
            config_search_current: 0,
            info_search_active: false,
            info_search_text: String::new(),
            should_quit: false,
            resources: resources.clone(),
            command_text: String::new(),
            command_suggestions: resources,
            command_suggestion_selected: 0,
            command_preview: None,
            describe_data: DescribeData::None,
            describe_scroll: 0,
            pending_action: None,
            last_key_press: None,
            auto_refresh_enabled,
            auto_refresh_interval,
            last_auto_refresh: None,
            autocomplete_suggestions: Vec::new(),
            autocomplete_selected: 0,
            autocomplete_visible: false,
            autocomplete_pool: Self::promql_builtins(),
            query_input: String::new(),
            query_results: Vec::new(),
            query_result_type: String::new(),
            selected_query_result_index: 0,
            query_input_active: false,
            query_history: Vec::new(),
            query_history_index: None,
            query_cursor_pos: 0,
            status_message: None,
            status_is_error: false,
            status_set_at: None,
            loading: false,
            spinner_frame: 0,
            multi_server_selected: Vec::new(),
            multi_server_cursor: 0,
            multi_server_mode_active: false,
            multi_server_alerts: Vec::new(),
            multi_server_query_results: Vec::new(),
            multi_server_query_result_type: String::new(),
            multi_server_flat_alert_index: 0,
            firing_seen: HashMap::new(),
            firing_last_seen: HashMap::new(),
            alert_toasts: Vec::new(),
            notification_sound_override: None,
            multi_server_flat_query_index: 0,
            multi_server_collapsed_alerts: HashSet::new(),
            multi_server_collapsed_query: HashSet::new(),
            pre_filter_collapsed_alerts: None,
            pre_filter_collapsed_query: None,
            exclusion_dialog_state: ExclusionDialogState::new(&[]),
            compiled_exclusions: Vec::new(),
            selected_exclusion_index: 0,
            cached_label_names: Vec::new(),
            cached_label_values: HashMap::new(),
            graph_visible: false,
            graph_data: Vec::new(),
            graph_time_min: 0.0,
            graph_time_max: 0.0,
            graph_value_min: 0.0,
            graph_value_max: 0.0,
            graph_legend_hovered: None,
        }
    }

    // ── New-alert notifications ─────────────────────────────────────────

    /// Whether an alert should raise a notification when it starts firing
    fn is_notifiable(&self, alert: &Alert, server: Option<&str>) -> bool {
        let severities = &self.tprom_config.notify_severities;
        alert.state == "firing"
            && (severities.is_empty() || severities.iter().any(|s| s == alert.severity()))
            && !self.is_alert_excluded(alert.name(), server)
    }

    /// Compare freshly fetched alerts with the previous fetch of each server and
    /// notify about alert names that started firing. Instances of an alert name
    /// that is already firing (e.g. a new failed job) don't count as new, and a
    /// name that fired again within `FLAP_WINDOW` is treated as flapping.
    /// `failed` servers keep their previous state.
    pub fn check_new_firing_alerts(&mut self, failed: &[String]) {
        const FLAP_WINDOW: std::time::Duration = std::time::Duration::from_secs(300);

        let mut fetched: Vec<(String, Vec<&Alert>)> = Vec::new();
        if self.multi_server_mode_active {
            for (server, alerts) in &self.multi_server_alerts {
                if failed.contains(server) {
                    continue;
                }
                let notifiable = alerts
                    .iter()
                    .filter(|a| self.is_notifiable(a, Some(server)))
                    .collect();
                fetched.push((server.clone(), notifiable));
            }
        } else {
            let notifiable = self
                .alerts
                .iter()
                .filter(|a| self.is_notifiable(a, None))
                .collect();
            fetched.push((self.current_server_name().to_string(), notifiable));
        }

        let now = std::time::Instant::now();
        let mut new_toasts = Vec::new();
        let mut updates = Vec::new();
        for (server, alerts) in fetched {
            let names: HashSet<String> = alerts.iter().map(|a| a.name().to_string()).collect();
            if let Some(previous) = self.firing_seen.get(&server) {
                for alert in &alerts {
                    let name = alert.name();
                    let flapping = self
                        .firing_last_seen
                        .get(&(server.clone(), name.to_string()))
                        .is_some_and(|t| now.duration_since(*t) < FLAP_WINDOW);
                    let already_toasted = new_toasts
                        .iter()
                        .any(|t: &AlertToast| t.server == server && t.alertname == name);
                    if !previous.contains(name) && !flapping && !already_toasted {
                        new_toasts.push(AlertToast::new(&server, alert));
                    }
                }
            }
            updates.push((server, names));
        }

        for (server, names) in updates {
            for name in &names {
                self.firing_last_seen
                    .insert((server.clone(), name.clone()), now);
            }
            self.firing_seen.insert(server, names);
        }

        if !new_toasts.is_empty() {
            if self.tprom_config.alert_sound {
                let sound = self
                    .notification_sound_override
                    .as_deref()
                    .unwrap_or(&self.tprom_config.alert_sound_name);
                crate::notify::play_alert_sound(sound, self.tprom_config.alert_sound_repeat);
            }
            new_toasts.append(&mut self.alert_toasts);
            new_toasts.truncate(100);
            self.alert_toasts = new_toasts;
        }
    }

    /// Persist the config and report `msg` (or the save error) in the status bar
    pub fn save_config_with_status(&mut self, msg: String) {
        match self.tprom_config.save() {
            Ok(()) => self.set_status(msg),
            Err(e) => self.set_error(format!("{} (not saved: {})", msg, e)),
        }
    }

    /// Step the auto-refresh interval up or down through common presets
    pub fn step_refresh_interval(&mut self, up: bool) {
        const PRESETS: [u64; 8] = [5, 10, 15, 30, 60, 120, 300, 600];
        let current = self.auto_refresh_interval.as_secs();
        let next = if up {
            PRESETS
                .iter()
                .copied()
                .find(|&p| p > current)
                .unwrap_or(600)
        } else {
            PRESETS
                .iter()
                .rev()
                .copied()
                .find(|&p| p < current)
                .unwrap_or(5)
        };
        self.auto_refresh_interval = std::time::Duration::from_secs(next);
        self.tprom_config.refresh_interval_secs = next;
        let state = if self.auto_refresh_enabled {
            "ON"
        } else {
            "OFF"
        };
        self.save_config_with_status(format!("Auto-refresh interval {}s ({})", next, state));
    }

    /// Set an informational status message (auto-clears after a few seconds)
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_is_error = false;
        self.status_set_at = Some(std::time::Instant::now());
    }

    /// Set an error status message (stays until next action, colored red)
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_is_error = true;
        self.status_set_at = Some(std::time::Instant::now());
    }

    /// Clear status message if it's old enough (non-errors clear after 3s)
    pub fn maybe_clear_status(&mut self) {
        if let Some(set_at) = self.status_set_at {
            if !self.status_is_error && set_at.elapsed() > std::time::Duration::from_secs(3) {
                self.status_message = None;
                self.status_set_at = None;
            }
        }
    }

    // ── Server management ────────────────────────────────────────────────

    pub fn needs_server_setup(&self) -> bool {
        self.tprom_config.servers.is_empty()
    }

    pub fn add_server_from_dialog(&mut self) -> Result<()> {
        let name = self.server_dialog_state.name.trim().to_string();
        let url = self.server_dialog_state.url.trim().to_string();

        if name.is_empty() {
            self.server_dialog_state
                .set_error("Name cannot be empty".to_string());
            return Ok(());
        }
        if url.is_empty() {
            self.server_dialog_state
                .set_error("URL cannot be empty".to_string());
            return Ok(());
        }

        let auth = self.server_dialog_state.get_auth_config();
        self.tprom_config
            .add_server(name.clone(), url.clone(), auth.clone())?;
        self.current_server = Some(ServerConfig { name, url, auth });
        Ok(())
    }

    pub fn delete_server(&mut self, server_name: &str) -> Result<()> {
        self.tprom_config.servers.retain(|s| s.name != server_name);
        self.tprom_config.save()?;
        Ok(())
    }

    pub fn current_server_name(&self) -> &str {
        self.current_server
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("No Server")
    }

    // ── Alert exclusion management ──────────────────────────────────────

    /// Rebuild compiled regex cache from config
    pub fn rebuild_exclusions(&mut self) {
        self.compiled_exclusions = self
            .tprom_config
            .alert_exclusions
            .iter()
            .filter_map(|exc| {
                Regex::new(&exc.pattern)
                    .ok()
                    .map(|re| (re, exc.servers.clone()))
            })
            .collect();
    }

    /// Check whether an alert name should be excluded for a given server.
    /// `server_name` is `None` for single-server mode.
    pub fn is_alert_excluded(&self, alert_name: &str, server_name: Option<&str>) -> bool {
        for (re, servers) in &self.compiled_exclusions {
            let applies = servers.is_empty()
                || server_name
                    .map(|sn| servers.iter().any(|s| s == sn))
                    .unwrap_or(true);
            if applies && re.is_match(alert_name) {
                return true;
            }
        }
        false
    }

    /// Save a new exclusion from the dialog state
    pub fn add_exclusion_from_dialog(&mut self) -> Result<()> {
        let pattern = self.exclusion_dialog_state.pattern.trim().to_string();
        if pattern.is_empty() {
            self.exclusion_dialog_state
                .set_error("Pattern cannot be empty".to_string());
            return Ok(());
        }
        if Regex::new(&pattern).is_err() {
            self.exclusion_dialog_state
                .set_error("Invalid regex pattern".to_string());
            return Ok(());
        }
        if self.exclusion_dialog_state.none_selected() {
            self.exclusion_dialog_state
                .set_error("Select at least one server".to_string());
            return Ok(());
        }

        let servers = self.exclusion_dialog_state.selected_server_names();
        let exclusion = AlertExclusion {
            pattern: pattern.clone(),
            servers,
        };

        if let Some(idx) = self.exclusion_dialog_state.editing_index {
            if idx < self.tprom_config.alert_exclusions.len() {
                self.tprom_config.alert_exclusions[idx] = exclusion;
            }
        } else {
            self.tprom_config.alert_exclusions.push(exclusion);
        }

        self.tprom_config.save()?;
        self.rebuild_exclusions();
        self.set_status(format!("Exclusion saved: {}", pattern));
        Ok(())
    }

    /// Delete an exclusion by index
    pub fn delete_exclusion(&mut self, index: usize) -> Result<()> {
        if index < self.tprom_config.alert_exclusions.len() {
            let removed = self.tprom_config.alert_exclusions.remove(index);
            self.tprom_config.save()?;
            self.rebuild_exclusions();
            if self.selected_exclusion_index >= self.tprom_config.alert_exclusions.len()
                && self.selected_exclusion_index > 0
            {
                self.selected_exclusion_index -= 1;
            }
            self.set_status(format!("Exclusion removed: {}", removed.pattern));
        }
        Ok(())
    }

    // ── Connection ───────────────────────────────────────────────────────

    pub fn connect(&mut self) -> Result<()> {
        if let Some(ref server) = self.current_server {
            self.client = Some(PrometheusClient::new(&server.url, server.auth.clone())?);
        }
        Ok(())
    }

    // ── Data fetching ────────────────────────────────────────────────────

    pub async fn fetch_targets(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            let data = client.fetch_targets().await?;
            self.targets = data.active_targets;
            if self.selected_target_index >= self.targets.len() {
                self.selected_target_index = 0;
            }
        }
        Ok(())
    }

    pub async fn fetch_alerts(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            let data = client.fetch_alerts().await?;
            self.alerts = data.alerts;
            self.check_new_firing_alerts(&[]);
            let filtered_len = self.apply_alert_filter().len();
            if filtered_len == 0 || self.selected_alert_index >= filtered_len {
                self.selected_alert_index = 0;
            }
        }
        Ok(())
    }

    pub async fn fetch_rules(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            let data = client.fetch_rules().await?;
            self.flat_rules.clear();
            for group in &data.groups {
                for rule in &group.rules {
                    self.flat_rules.push((group.name.clone(), rule.clone()));
                }
            }
            self.rule_groups = data.groups;
            if self.selected_rule_index >= self.flat_rules.len() {
                self.selected_rule_index = 0;
            }
        }
        Ok(())
    }

    pub async fn fetch_metrics(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            let names = client.fetch_metric_names().await?;
            let meta = client.fetch_metadata().await.unwrap_or_default();
            self.metric_names = names;
            self.metadata = meta;
            self.apply_metric_filter();
        }
        Ok(())
    }

    pub async fn fetch_labels(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            self.label_names = client.fetch_label_names().await?;
            self.label_values.clear();
            self.selected_label_index = 0;
            self.selected_label_value_index = 0;
            // Fetch values for first label if any
            if !self.label_names.is_empty() {
                let values = client
                    .fetch_label_values(&self.label_names[0])
                    .await
                    .unwrap_or_default();
                self.label_values = values;
            }
        }
        Ok(())
    }

    pub async fn fetch_label_values_for_current(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            if self.selected_label_index < self.label_names.len() {
                let label = &self.label_names[self.selected_label_index];
                self.label_values = client.fetch_label_values(label).await.unwrap_or_default();
                self.selected_label_value_index = 0;
            }
        }
        Ok(())
    }

    pub async fn fetch_tsdb_status(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            self.tsdb_status = Some(client.fetch_tsdb_status().await?);
            self.tsdb_scroll = 0;
        }
        Ok(())
    }

    pub async fn fetch_config(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            let data = client.fetch_config().await?;
            self.config_yaml = data.yaml;
            self.config_scroll = 0;
        }
        Ok(())
    }

    pub async fn fetch_info(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            self.build_info = client.fetch_build_info().await.ok();
            self.runtime_info = client.fetch_runtime_info().await.ok();
            let flags_map = client.fetch_flags().await.unwrap_or_default();
            let mut flags: Vec<(String, String)> = flags_map.into_iter().collect();
            flags.sort_by(|a, b| a.0.cmp(&b.0));
            self.flags = flags;
            self.info_scroll = 0;
        }
        Ok(())
    }

    pub fn describe_target(&mut self) {
        let filtered = self.apply_target_filter();
        if !filtered.is_empty() && self.selected_target_index < filtered.len() {
            let target = filtered[self.selected_target_index].clone();
            self.describe_data = DescribeData::Target(target);
            self.describe_scroll = 0;
        }
    }

    pub fn describe_alert(&mut self) {
        if self.multi_server_mode_active {
            // Multi-server mode: walk the flat tree to find the alert at the current index
            let filtered = self.filtered_multi_server_alerts();
            let mut pos = 0;
            for (name, alerts) in &filtered {
                let is_collapsed = self.multi_server_collapsed_alerts.contains(name);
                if self.multi_server_flat_alert_index == pos {
                    // This is a header row, nothing to describe
                    return;
                }
                pos += 1; // past header
                if !is_collapsed {
                    for alert in alerts {
                        if self.multi_server_flat_alert_index == pos {
                            self.describe_data = DescribeData::Alert((*alert).clone());
                            self.describe_scroll = 0;
                            return;
                        }
                        pos += 1;
                    }
                }
            }
        } else {
            let filtered = self.apply_alert_filter();
            if !filtered.is_empty() && self.selected_alert_index < filtered.len() {
                let alert = filtered[self.selected_alert_index].clone();
                self.describe_data = DescribeData::Alert(alert);
                self.describe_scroll = 0;
            }
        }
    }

    pub fn describe_rule(&mut self) {
        let filtered = self.apply_rule_filter();
        if !filtered.is_empty() && self.selected_rule_index < filtered.len() {
            let (_, rule) = filtered[self.selected_rule_index].clone();
            self.describe_data = DescribeData::Rule(rule);
            self.describe_scroll = 0;
        }
    }

    pub async fn describe_metric(&mut self) -> Result<()> {
        if self.filtered_metrics.is_empty()
            || self.selected_metric_index >= self.filtered_metrics.len()
        {
            return Ok(());
        }
        let name = self.filtered_metrics[self.selected_metric_index].clone();

        let info = if let Some(metas) = self.metadata.get(&name) {
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

        // Run instant query for this metric
        let results = if let Some(ref client) = self.client {
            client
                .query_instant(&name)
                .await
                .map(|d| d.result)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        self.describe_data = DescribeData::Metric {
            name,
            info,
            results,
        };
        self.describe_scroll = 0;
        Ok(())
    }

    /// Returns the label name that needs values fetched, if we're in LabelValue context
    /// and the values aren't cached yet.
    pub fn pending_label_values_fetch(&self) -> Option<String> {
        if let LabelContext::LabelValue { ref label_name, .. } = self.label_context() {
            if !label_name.is_empty() && !self.cached_label_values.contains_key(label_name) {
                return Some(label_name.clone());
            }
        }
        None
    }

    /// Fetch label names into the autocomplete cache
    pub async fn fetch_cached_label_names(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            self.cached_label_names = client.fetch_label_names().await.unwrap_or_default();
        }
        Ok(())
    }

    /// Fetch label values for a given label name into the cache (if not already cached)
    pub async fn fetch_cached_label_values(&mut self, label_name: &str) -> Result<()> {
        if self.cached_label_values.contains_key(label_name) {
            return Ok(());
        }
        if let Some(ref client) = self.client {
            let values = client
                .fetch_label_values(label_name)
                .await
                .unwrap_or_default();
            self.cached_label_values
                .insert(label_name.to_string(), values);
        }
        Ok(())
    }

    /// Execute a range query for the current expression and populate graph data.
    /// Uses a 1-hour window ending now with auto-calculated step.
    pub async fn execute_graph_query(&mut self) -> Result<()> {
        let expr = self.query_input.trim().to_string();
        if expr.is_empty() {
            return Ok(());
        }

        if let Some(ref client) = self.client {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            let start = now - 3600.0; // 1 hour ago
            let end = now;
            // ~240 data points for a nice graph
            let step = (end - start) / 240.0;

            let data = client.query_range(&expr, start, end, step).await?;

            self.graph_data.clear();
            self.graph_time_min = start;
            self.graph_time_max = end;
            self.graph_value_min = f64::MAX;
            self.graph_value_max = f64::MIN;

            for result in &data.result {
                let label = result.label_string();
                let mut points: Vec<(f64, f64)> = Vec::new();

                if let Some(ref values) = result.values {
                    for (ts, val_str) in values {
                        if let Ok(val) = val_str.parse::<f64>() {
                            if val.is_finite() {
                                if val < self.graph_value_min {
                                    self.graph_value_min = val;
                                }
                                if val > self.graph_value_max {
                                    self.graph_value_max = val;
                                }
                                points.push((*ts, val));
                            }
                        }
                    }
                }

                if !points.is_empty() {
                    self.graph_data.push((label, points));
                }
            }

            // Add some padding to value range
            if self.graph_value_min == self.graph_value_max {
                self.graph_value_min -= 1.0;
                self.graph_value_max += 1.0;
            } else {
                let range = self.graph_value_max - self.graph_value_min;
                self.graph_value_min -= range * 0.05;
                self.graph_value_max += range * 0.05;
            }

            self.graph_visible = true;
        }
        Ok(())
    }

    pub fn describe_query_result(&mut self) {
        if !self.query_results.is_empty()
            && self.selected_query_result_index < self.query_results.len()
        {
            let result = self.query_results[self.selected_query_result_index].clone();
            self.describe_data = DescribeData::QueryResult(result);
            self.describe_scroll = 0;
        }
    }

    // ── Autocomplete ──────────────────────────────────────────────────────

    fn promql_builtins() -> Vec<AutocompleteItem> {
        let functions = [
            "abs",
            "absent",
            "absent_over_time",
            "avg",
            "avg_over_time",
            "ceil",
            "changes",
            "clamp",
            "clamp_max",
            "clamp_min",
            "count",
            "count_over_time",
            "count_values",
            "day_of_month",
            "day_of_week",
            "day_of_year",
            "days_in_month",
            "delta",
            "deriv",
            "exp",
            "floor",
            "group",
            "histogram_count",
            "histogram_fraction",
            "histogram_quantile",
            "histogram_sum",
            "holt_winters",
            "hour",
            "idelta",
            "increase",
            "irate",
            "label_join",
            "label_replace",
            "last_over_time",
            "ln",
            "log2",
            "log10",
            "max",
            "max_over_time",
            "min",
            "min_over_time",
            "minute",
            "month",
            "predict_linear",
            "present_over_time",
            "quantile",
            "quantile_over_time",
            "rate",
            "resets",
            "round",
            "scalar",
            "sgn",
            "sort",
            "sort_desc",
            "sqrt",
            "stddev",
            "stddev_over_time",
            "stdvar",
            "stdvar_over_time",
            "sum",
            "sum_over_time",
            "time",
            "timestamp",
            "topk",
            "bottomk",
            "vector",
            "year",
        ];

        let keywords = [
            "by",
            "without",
            "on",
            "ignoring",
            "group_left",
            "group_right",
            "bool",
            "offset",
        ];

        let mut pool = Vec::new();
        for f in &functions {
            pool.push(AutocompleteItem {
                text: format!("{}(", f),
                kind: AutocompleteKind::Function,
            });
        }
        for k in &keywords {
            pool.push(AutocompleteItem {
                text: k.to_string(),
                kind: AutocompleteKind::Keyword,
            });
        }
        pool
    }

    /// Rebuild the autocomplete pool with metric names + builtins
    pub fn rebuild_autocomplete_pool(&mut self) {
        let mut pool = Self::promql_builtins();
        for name in &self.metric_names {
            pool.push(AutocompleteItem {
                text: name.clone(),
                kind: AutocompleteKind::Metric,
            });
        }
        self.autocomplete_pool = pool;
    }

    /// Determine the autocomplete context by analyzing text before the cursor.
    /// Parses whether we are inside `{}` and at what position.
    fn label_context(&self) -> LabelContext {
        let input = &self.query_input;
        if input.is_empty() {
            return LabelContext::None;
        }

        // Find if we're inside braces by scanning for unmatched `{`
        let mut brace_depth = 0i32;
        let mut last_open_brace = None;
        for (i, ch) in input.char_indices() {
            if ch == '{' {
                brace_depth += 1;
                last_open_brace = Some(i);
            } else if ch == '}' {
                brace_depth -= 1;
            }
        }

        if brace_depth <= 0 || last_open_brace.is_none() {
            return LabelContext::None;
        }

        // We're inside braces. Parse the content after the last `{`.
        let brace_pos = last_open_brace.unwrap();
        let inside = &input[brace_pos + 1..];

        // Split by `,` to get the current matcher being typed
        let current_matcher = inside.rsplit(',').next().unwrap_or("").trim_start();

        // Parse the current matcher to determine context
        // Possible states:
        //   ""           -> just typed `{` or `,` -> LabelName
        //   "pod"        -> typing label name     -> LabelName
        //   "pod="       -> typed operator         -> LabelValue
        //   "pod=~"      -> typed operator         -> LabelValue
        //   "pod!="      -> typed operator         -> LabelValue
        //   "pod!~"      -> typed operator         -> LabelValue
        //   "pod=\"val"  -> typing value           -> LabelValue

        // Check for operator patterns
        let op_patterns: &[&str] = &["=~", "!~", "!=", "="];
        for op in op_patterns {
            if let Some(pos) = current_matcher.find(op) {
                let label_name = current_matcher[..pos].trim().to_string();
                let after_op = &current_matcher[pos + op.len()..];
                // Strip leading quote from value prefix
                let prefix = after_op.trim_start_matches('"').to_string();
                return LabelContext::LabelValue { label_name, prefix };
            }
        }

        // No operator found yet. Check if we have a complete label name
        // followed by something that could be the start of an operator
        let trimmed = current_matcher.trim();
        if trimmed.is_empty() {
            return LabelContext::LabelName {
                prefix: String::new(),
            };
        }

        // Check if last char suggests we need an operator
        let last_char = trimmed.chars().last().unwrap();
        if last_char == '!' {
            // Started typing `!=` or `!~`
            let label_name = trimmed.trim_end_matches('!').trim().to_string();
            return LabelContext::Operator { label_name };
        }

        // If it's all identifier chars, it's a label name being typed
        if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // Could be a complete label name needing operator, or partial.
            // We offer label name completion; after accepting, user types operator.
            return LabelContext::LabelName {
                prefix: trimmed.to_string(),
            };
        }

        LabelContext::None
    }

    /// Extract the current token being typed (the word at the cursor) — for outside-braces context
    fn current_token(&self) -> &str {
        let input = &self.query_input;
        if input.is_empty() {
            return "";
        }
        // Walk backwards from end to find the start of the current token
        let bytes = input.as_bytes();
        let mut start = bytes.len();
        for i in (0..bytes.len()).rev() {
            let ch = bytes[i] as char;
            if ch == '('
                || ch == ')'
                || ch == '{'
                || ch == '}'
                || ch == ','
                || ch == ' '
                || ch == '['
                || ch == ']'
                || ch == '"'
                || ch == '+'
                || ch == '-'
                || ch == '*'
                || ch == '/'
                || ch == '='
                || ch == '!'
                || ch == '~'
                || ch == '>'
                || ch == '<'
            {
                break;
            }
            start = i;
        }
        &input[start..]
    }

    /// Update autocomplete suggestions based on current input context
    pub fn update_autocomplete(&mut self) {
        let ctx = self.label_context();

        match ctx {
            LabelContext::LabelName { ref prefix } => {
                let prefix_lower = prefix.to_lowercase();
                self.autocomplete_suggestions = self
                    .cached_label_names
                    .iter()
                    .filter(|name| {
                        let name_lower = name.to_lowercase();
                        if prefix_lower.is_empty() {
                            true
                        } else {
                            name_lower.starts_with(&prefix_lower) && name_lower != prefix_lower
                        }
                    })
                    .take(10)
                    .map(|name| AutocompleteItem {
                        text: name.clone(),
                        kind: AutocompleteKind::LabelName,
                    })
                    .collect();
            }
            LabelContext::Operator { .. } => {
                self.autocomplete_suggestions = vec![
                    AutocompleteItem {
                        text: "=".to_string(),
                        kind: AutocompleteKind::Operator,
                    },
                    AutocompleteItem {
                        text: "!=".to_string(),
                        kind: AutocompleteKind::Operator,
                    },
                    AutocompleteItem {
                        text: "=~".to_string(),
                        kind: AutocompleteKind::Operator,
                    },
                    AutocompleteItem {
                        text: "!~".to_string(),
                        kind: AutocompleteKind::Operator,
                    },
                ];
            }
            LabelContext::LabelValue {
                ref label_name,
                ref prefix,
            } => {
                let values = self
                    .cached_label_values
                    .get(label_name)
                    .cloned()
                    .unwrap_or_default();
                let prefix_lower = prefix.to_lowercase();
                self.autocomplete_suggestions = values
                    .iter()
                    .filter(|v| {
                        let v_lower = v.to_lowercase();
                        if prefix_lower.is_empty() {
                            true
                        } else {
                            v_lower.starts_with(&prefix_lower) && v_lower != prefix_lower
                        }
                    })
                    .take(10)
                    .map(|v| AutocompleteItem {
                        text: v.clone(),
                        kind: AutocompleteKind::LabelValue,
                    })
                    .collect();
            }
            LabelContext::None => {
                // Original behavior: match from autocomplete_pool
                let token = self.current_token().to_lowercase();
                if token.is_empty() {
                    self.autocomplete_suggestions.clear();
                    self.autocomplete_visible = false;
                    self.autocomplete_selected = 0;
                    return;
                }

                self.autocomplete_suggestions = self
                    .autocomplete_pool
                    .iter()
                    .filter(|item| {
                        let text_lower = item.text.to_lowercase();
                        text_lower.starts_with(&token) && text_lower != token
                    })
                    .take(10)
                    .cloned()
                    .collect();
            }
        }

        self.autocomplete_visible = !self.autocomplete_suggestions.is_empty();
        if self.autocomplete_selected >= self.autocomplete_suggestions.len() {
            self.autocomplete_selected = 0;
        }
    }

    /// Accept the currently selected autocomplete suggestion
    pub fn accept_autocomplete(&mut self) {
        if !self.autocomplete_visible || self.autocomplete_suggestions.is_empty() {
            return;
        }
        let suggestion = self.autocomplete_suggestions[self.autocomplete_selected].clone();
        let ctx = self.label_context();

        match ctx {
            LabelContext::LabelName { ref prefix } => {
                // Replace the typed prefix with the full label name
                if !prefix.is_empty() {
                    let input_len = self.query_input.len();
                    self.query_input.truncate(input_len - prefix.len());
                }
                self.query_input.push_str(&suggestion.text);
            }
            LabelContext::Operator { .. } => {
                // Remove any partial operator chars (e.g. `!`) then insert full operator + `"`
                let trimmed = self.query_input.trim_end().to_string();
                if trimmed.ends_with('!') {
                    self.query_input = trimmed[..trimmed.len() - 1].to_string();
                }
                self.query_input.push_str(&suggestion.text);
                self.query_input.push('"');
            }
            LabelContext::LabelValue { ref prefix, .. } => {
                // Replace the typed value prefix with the full value + closing quote
                if !prefix.is_empty() {
                    let input_len = self.query_input.len();
                    self.query_input.truncate(input_len - prefix.len());
                }
                self.query_input.push_str(&suggestion.text);
                self.query_input.push('"');
            }
            LabelContext::None => {
                // Original behavior: replace current token
                let token = self.current_token().to_string();
                if !token.is_empty() {
                    let input_len = self.query_input.len();
                    let token_len = token.len();
                    self.query_input.truncate(input_len - token_len);
                }
                self.query_input.push_str(&suggestion.text);
            }
        }

        self.query_cursor_pos = self.query_input.len();
        self.autocomplete_suggestions.clear();
        self.autocomplete_visible = false;
        self.autocomplete_selected = 0;
    }

    // ── Query (PromQL REPL) ─────────────────────────────────────────────

    pub async fn execute_query(&mut self) -> Result<()> {
        let expr = self.query_input.trim().to_string();
        if expr.is_empty() {
            return Ok(());
        }

        // Add to history (avoid duplicates at the end)
        if self.query_history.last().map(|s| s.as_str()) != Some(&expr) {
            self.query_history.push(expr.clone());
        }
        self.query_history_index = None;

        if let Some(ref client) = self.client {
            let data = client.query_instant(&expr).await?;
            self.query_result_type = data.result_type;
            self.query_results = data.result;
            self.selected_query_result_index = 0;
            // Switch focus to results after successful query
            if !self.query_results.is_empty() {
                self.query_input_active = false;
            }
        }
        Ok(())
    }

    pub fn query_history_prev(&mut self) {
        if self.query_history.is_empty() {
            return;
        }
        match self.query_history_index {
            None => {
                self.query_history_index = Some(self.query_history.len() - 1);
            }
            Some(0) => {} // already at oldest
            Some(idx) => {
                self.query_history_index = Some(idx - 1);
            }
        }
        if let Some(idx) = self.query_history_index {
            self.query_input = self.query_history[idx].clone();
            self.query_cursor_pos = self.query_input.len();
        }
    }

    pub fn query_history_next(&mut self) {
        if self.query_history.is_empty() {
            return;
        }
        match self.query_history_index {
            None => {}
            Some(idx) if idx >= self.query_history.len() - 1 => {
                // Past end of history, clear to empty
                self.query_history_index = None;
                self.query_input.clear();
                self.query_cursor_pos = 0;
            }
            Some(idx) => {
                self.query_history_index = Some(idx + 1);
                self.query_input = self.query_history[idx + 1].clone();
                self.query_cursor_pos = self.query_input.len();
            }
        }
    }

    // ── Filter / search ──────────────────────────────────────────────────

    pub fn apply_metric_filter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_metrics = self.metric_names.clone();
        } else {
            let filter = self.filter_text.to_lowercase();
            self.filtered_metrics = self
                .metric_names
                .iter()
                .filter(|n| n.to_lowercase().contains(&filter))
                .cloned()
                .collect();
        }
        if self.selected_metric_index >= self.filtered_metrics.len() {
            self.selected_metric_index = 0;
        }
    }

    pub fn apply_target_filter(&self) -> Vec<&ActiveTarget> {
        if self.filter_text.is_empty() {
            self.targets.iter().collect()
        } else {
            let filter = self.filter_text.to_lowercase();
            self.targets
                .iter()
                .filter(|t| {
                    t.job().to_lowercase().contains(&filter)
                        || t.instance().to_lowercase().contains(&filter)
                })
                .collect()
        }
    }

    pub fn apply_alert_filter(&self) -> Vec<&Alert> {
        let server = self.current_server.as_ref().map(|s| s.name.as_str());
        if self.filter_text.is_empty() {
            self.alerts
                .iter()
                .filter(|a| !self.is_alert_excluded(a.name(), server))
                .collect()
        } else {
            let filter = self.filter_text.to_lowercase();
            self.alerts
                .iter()
                .filter(|a| {
                    a.name().to_lowercase().contains(&filter)
                        && !self.is_alert_excluded(a.name(), server)
                })
                .collect()
        }
    }

    pub fn apply_rule_filter(&self) -> Vec<&(String, Rule)> {
        if self.filter_text.is_empty() {
            self.flat_rules.iter().collect()
        } else {
            let filter = self.filter_text.to_lowercase();
            self.flat_rules
                .iter()
                .filter(|(g, r)| {
                    g.to_lowercase().contains(&filter) || r.name.to_lowercase().contains(&filter)
                })
                .collect()
        }
    }

    /// Return multi-server alerts filtered by name and state, with exclusions applied.
    /// Each group retains its server name but only matching alerts.
    /// Groups with zero matching alerts are excluded.
    pub fn filtered_multi_server_alerts(&self) -> Vec<(String, Vec<&Alert>)> {
        if self.filter_text.is_empty() {
            return self
                .multi_server_alerts
                .iter()
                .filter_map(|(name, alerts)| {
                    let filtered: Vec<&Alert> = alerts
                        .iter()
                        .filter(|a| !self.is_alert_excluded(a.name(), Some(name)))
                        .collect();
                    if filtered.is_empty() {
                        None
                    } else {
                        Some((name.clone(), filtered))
                    }
                })
                .collect();
        }
        let filter = self.filter_text.to_lowercase();
        self.multi_server_alerts
            .iter()
            .filter_map(|(name, alerts)| {
                let filtered: Vec<&Alert> = alerts
                    .iter()
                    .filter(|a| {
                        (a.name().to_lowercase().contains(&filter)
                            || a.state.to_lowercase().contains(&filter))
                            && !self.is_alert_excluded(a.name(), Some(name))
                    })
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some((name.clone(), filtered))
                }
            })
            .collect()
    }

    /// Return multi-server query results filtered by label string.
    /// Groups with zero matching results are excluded.
    pub fn filtered_multi_server_query(&self) -> Vec<(String, Vec<&QueryResult>)> {
        if self.filter_text.is_empty() {
            return self
                .multi_server_query_results
                .iter()
                .map(|(name, results)| (name.clone(), results.iter().collect()))
                .collect();
        }
        let filter = self.filter_text.to_lowercase();
        self.multi_server_query_results
            .iter()
            .filter_map(|(name, results)| {
                let filtered: Vec<&QueryResult> = results
                    .iter()
                    .filter(|r| r.label_string().to_lowercase().contains(&filter))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some((name.clone(), filtered))
                }
            })
            .collect()
    }

    // Config search helpers
    pub fn update_config_search(&mut self) {
        self.config_search_matches.clear();
        self.config_search_current = 0;
        if self.config_search_text.is_empty() {
            return;
        }
        let search_lower = self.config_search_text.to_lowercase();
        for (idx, line) in self.config_yaml.lines().enumerate() {
            if line.to_lowercase().contains(&search_lower) {
                self.config_search_matches.push(idx);
            }
        }
        if !self.config_search_matches.is_empty() {
            self.config_scroll = self.config_search_matches[0];
        }
    }

    pub fn config_search_next(&mut self) {
        if self.config_search_matches.is_empty() {
            return;
        }
        self.config_search_current =
            (self.config_search_current + 1) % self.config_search_matches.len();
        self.config_scroll = self.config_search_matches[self.config_search_current];
    }

    pub fn config_search_prev(&mut self) {
        if self.config_search_matches.is_empty() {
            return;
        }
        if self.config_search_current == 0 {
            self.config_search_current = self.config_search_matches.len() - 1;
        } else {
            self.config_search_current -= 1;
        }
        self.config_scroll = self.config_search_matches[self.config_search_current];
    }

    // ── Navigation helpers ───────────────────────────────────────────────

    pub fn nav_down(&mut self) {
        match self.active_resource.as_str() {
            "targets" => {
                let filtered_len = self.apply_target_filter().len();
                if filtered_len > 0 && self.selected_target_index < filtered_len - 1 {
                    self.selected_target_index += 1;
                }
            }
            "alerts" => {
                if self.multi_server_mode_active {
                    let len = self.multi_server_alert_flat_len();
                    if len > 0 && self.multi_server_flat_alert_index < len - 1 {
                        self.multi_server_flat_alert_index += 1;
                        self.multi_alert_table_state
                            .select(Some(self.multi_server_flat_alert_index));
                    }
                } else {
                    let filtered_len = self.apply_alert_filter().len();
                    if filtered_len > 0 && self.selected_alert_index < filtered_len - 1 {
                        self.selected_alert_index += 1;
                        self.alert_table_state
                            .select(Some(self.selected_alert_index));
                    }
                }
            }
            "rules" => {
                let filtered_len = self.apply_rule_filter().len();
                if filtered_len > 0 && self.selected_rule_index < filtered_len - 1 {
                    self.selected_rule_index += 1;
                }
            }
            "metrics" => {
                if !self.filtered_metrics.is_empty()
                    && self.selected_metric_index < self.filtered_metrics.len() - 1
                {
                    self.selected_metric_index += 1;
                }
            }
            "labels" => {
                if self.label_focus_right {
                    if !self.label_values.is_empty()
                        && self.selected_label_value_index < self.label_values.len() - 1
                    {
                        self.selected_label_value_index += 1;
                    }
                } else if !self.label_names.is_empty()
                    && self.selected_label_index < self.label_names.len() - 1
                {
                    self.selected_label_index += 1;
                }
            }
            "query" => {
                if !self.query_input_active {
                    if self.multi_server_mode_active {
                        let len = self.multi_server_query_flat_len();
                        if len > 0 && self.multi_server_flat_query_index < len - 1 {
                            self.multi_server_flat_query_index += 1;
                        }
                    } else if !self.query_results.is_empty()
                        && self.selected_query_result_index < self.query_results.len() - 1
                    {
                        self.selected_query_result_index += 1;
                    }
                }
            }
            "servers" => {
                if !self.tprom_config.servers.is_empty()
                    && self.selected_server_index < self.tprom_config.servers.len() - 1
                {
                    self.selected_server_index += 1;
                }
            }
            "alertexclusion" => {
                if !self.tprom_config.alert_exclusions.is_empty()
                    && self.selected_exclusion_index < self.tprom_config.alert_exclusions.len() - 1
                {
                    self.selected_exclusion_index += 1;
                }
            }
            "tsdb" => {
                self.tsdb_scroll = self.tsdb_scroll.saturating_add(1);
            }
            "config" => {
                self.config_scroll = self.config_scroll.saturating_add(1);
            }
            "info" => {
                self.info_scroll = self.info_scroll.saturating_add(1);
            }
            _ => {}
        }
    }

    pub fn nav_up(&mut self) {
        match self.active_resource.as_str() {
            "targets" => {
                if self.selected_target_index > 0 {
                    self.selected_target_index -= 1;
                }
            }
            "alerts" => {
                if self.multi_server_mode_active {
                    if self.multi_server_flat_alert_index > 0 {
                        self.multi_server_flat_alert_index -= 1;
                        self.multi_alert_table_state
                            .select(Some(self.multi_server_flat_alert_index));
                    }
                } else if self.selected_alert_index > 0 {
                    self.selected_alert_index -= 1;
                    self.alert_table_state
                        .select(Some(self.selected_alert_index));
                }
            }
            "rules" => {
                if self.selected_rule_index > 0 {
                    self.selected_rule_index -= 1;
                }
            }
            "metrics" => {
                if self.selected_metric_index > 0 {
                    self.selected_metric_index -= 1;
                }
            }
            "labels" => {
                if self.label_focus_right {
                    if self.selected_label_value_index > 0 {
                        self.selected_label_value_index -= 1;
                    }
                } else if self.selected_label_index > 0 {
                    self.selected_label_index -= 1;
                }
            }
            "query" => {
                if !self.query_input_active {
                    if self.multi_server_mode_active {
                        if self.multi_server_flat_query_index > 0 {
                            self.multi_server_flat_query_index -= 1;
                        }
                    } else if self.selected_query_result_index > 0 {
                        self.selected_query_result_index -= 1;
                    }
                }
            }
            "servers" => {
                if self.selected_server_index > 0 {
                    self.selected_server_index -= 1;
                }
            }
            "alertexclusion" => {
                if self.selected_exclusion_index > 0 {
                    self.selected_exclusion_index -= 1;
                }
            }
            "tsdb" => {
                self.tsdb_scroll = self.tsdb_scroll.saturating_sub(1);
            }
            "config" => {
                self.config_scroll = self.config_scroll.saturating_sub(1);
            }
            "info" => {
                self.info_scroll = self.info_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub fn go_to_top(&mut self) {
        match self.active_resource.as_str() {
            "targets" => self.selected_target_index = 0,
            "alerts" => {
                if self.multi_server_mode_active {
                    self.multi_server_flat_alert_index = 0;
                    self.multi_alert_table_state.select(Some(0));
                } else {
                    self.selected_alert_index = 0;
                    self.alert_table_state.select(Some(0));
                }
            }
            "rules" => self.selected_rule_index = 0,
            "metrics" => self.selected_metric_index = 0,
            "labels" => {
                if self.label_focus_right {
                    self.selected_label_value_index = 0;
                } else {
                    self.selected_label_index = 0;
                }
            }
            "query" => {
                if self.multi_server_mode_active {
                    self.multi_server_flat_query_index = 0;
                } else {
                    self.selected_query_result_index = 0;
                }
            }
            "servers" => self.selected_server_index = 0,
            "alertexclusion" => self.selected_exclusion_index = 0,
            "tsdb" => self.tsdb_scroll = 0,
            "config" => self.config_scroll = 0,
            "info" => self.info_scroll = 0,
            _ => {}
        }
    }

    pub fn go_to_bottom(&mut self) {
        match self.active_resource.as_str() {
            "targets" => {
                let filtered_len = self.apply_target_filter().len();
                if filtered_len > 0 {
                    self.selected_target_index = filtered_len - 1;
                }
            }
            "alerts" => {
                if self.multi_server_mode_active {
                    let len = self.multi_server_alert_flat_len();
                    if len > 0 {
                        self.multi_server_flat_alert_index = len - 1;
                        self.multi_alert_table_state.select(Some(len - 1));
                    }
                } else {
                    let filtered_len = self.apply_alert_filter().len();
                    if filtered_len > 0 {
                        self.selected_alert_index = filtered_len - 1;
                        self.alert_table_state
                            .select(Some(self.selected_alert_index));
                    }
                }
            }
            "rules" => {
                let filtered_len = self.apply_rule_filter().len();
                if filtered_len > 0 {
                    self.selected_rule_index = filtered_len - 1;
                }
            }
            "metrics" => {
                if !self.filtered_metrics.is_empty() {
                    self.selected_metric_index = self.filtered_metrics.len() - 1;
                }
            }
            "labels" => {
                if self.label_focus_right {
                    if !self.label_values.is_empty() {
                        self.selected_label_value_index = self.label_values.len() - 1;
                    }
                } else if !self.label_names.is_empty() {
                    self.selected_label_index = self.label_names.len() - 1;
                }
            }
            "query" => {
                if self.multi_server_mode_active {
                    let len = self.multi_server_query_flat_len();
                    if len > 0 {
                        self.multi_server_flat_query_index = len - 1;
                    }
                } else if !self.query_results.is_empty() {
                    self.selected_query_result_index = self.query_results.len() - 1;
                }
            }
            "servers" => {
                if !self.tprom_config.servers.is_empty() {
                    self.selected_server_index = self.tprom_config.servers.len() - 1;
                }
            }
            "alertexclusion" => {
                if !self.tprom_config.alert_exclusions.is_empty() {
                    self.selected_exclusion_index = self.tprom_config.alert_exclusions.len() - 1;
                }
            }
            "tsdb" | "config" | "info" => {
                // scroll views - set to large number, rendering will clamp
                match self.active_resource.as_str() {
                    "tsdb" => self.tsdb_scroll = 999999,
                    "config" => self.config_scroll = 999999,
                    "info" => self.info_scroll = 999999,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn describe_scroll_down(&mut self) {
        self.describe_scroll = self.describe_scroll.saturating_add(1);
    }

    pub fn describe_scroll_up(&mut self) {
        self.describe_scroll = self.describe_scroll.saturating_sub(1);
    }

    // ── Command palette ──────────────────────────────────────────────────

    pub fn update_command_suggestions(&mut self) {
        let typed = self.command_text.to_lowercase();
        self.command_suggestions = self
            .resources
            .iter()
            .filter(|r| r.command.to_lowercase().contains(&typed))
            .cloned()
            .collect();

        if self.command_suggestion_selected >= self.command_suggestions.len() {
            self.command_suggestion_selected = 0;
        }

        self.command_preview = self.command_suggestions.first().map(|r| r.command.clone());
    }

    // ── Multi-server helpers ─────────────────────────────────────────────

    /// Total flat rows for multi-server alerts (respects collapsed + filter)
    pub fn multi_server_alert_flat_len(&self) -> usize {
        let filtered = self.filtered_multi_server_alerts();
        filtered
            .iter()
            .map(|(name, alerts)| {
                if self.multi_server_collapsed_alerts.contains(name) {
                    1
                } else {
                    1 + alerts.len()
                }
            })
            .sum()
    }

    /// Total flat rows for multi-server query results (respects collapsed + filter)
    pub fn multi_server_query_flat_len(&self) -> usize {
        let filtered = self.filtered_multi_server_query();
        filtered
            .iter()
            .map(|(name, results)| {
                if self.multi_server_collapsed_query.contains(name) {
                    1
                } else {
                    1 + results.len()
                }
            })
            .sum()
    }

    /// Check if a given flat index is a server header row (for alerts)
    pub fn multi_server_alert_is_header(&self, flat_idx: usize) -> bool {
        let filtered = self.filtered_multi_server_alerts();
        let mut pos = 0;
        for (name, alerts) in &filtered {
            if flat_idx == pos {
                return true;
            }
            let children = if self.multi_server_collapsed_alerts.contains(name) {
                0
            } else {
                alerts.len()
            };
            pos += 1 + children;
        }
        false
    }

    /// Check if a given flat index is a server header row (for query results)
    pub fn multi_server_query_is_header(&self, flat_idx: usize) -> bool {
        let filtered = self.filtered_multi_server_query();
        let mut pos = 0;
        for (name, results) in &filtered {
            if flat_idx == pos {
                return true;
            }
            let children = if self.multi_server_collapsed_query.contains(name) {
                0
            } else {
                results.len()
            };
            pos += 1 + children;
        }
        false
    }

    /// Get the server name at a given flat index (alerts)
    pub fn multi_server_alert_server_at(&self, flat_idx: usize) -> Option<String> {
        let filtered = self.filtered_multi_server_alerts();
        let mut pos = 0;
        for (name, alerts) in &filtered {
            if flat_idx == pos {
                return Some(name.clone());
            }
            let children = if self.multi_server_collapsed_alerts.contains(name) {
                0
            } else {
                alerts.len()
            };
            pos += 1 + children;
        }
        None
    }

    /// Get the server name at a given flat index (query)
    pub fn multi_server_query_server_at(&self, flat_idx: usize) -> Option<String> {
        let filtered = self.filtered_multi_server_query();
        let mut pos = 0;
        for (name, results) in &filtered {
            if flat_idx == pos {
                return Some(name.clone());
            }
            let children = if self.multi_server_collapsed_query.contains(name) {
                0
            } else {
                results.len()
            };
            pos += 1 + children;
        }
        None
    }

    /// Toggle collapse/expand for a server group (alerts)
    pub fn multi_server_alert_toggle_collapse(&mut self, server_name: &str) {
        if !self.multi_server_collapsed_alerts.remove(server_name) {
            self.multi_server_collapsed_alerts
                .insert(server_name.to_string());
        }
    }

    /// Toggle collapse/expand for a server group (query)
    pub fn multi_server_query_toggle_collapse(&mut self, server_name: &str) {
        if !self.multi_server_collapsed_query.remove(server_name) {
            self.multi_server_collapsed_query
                .insert(server_name.to_string());
        }
    }

    /// Toggle server selection for multi-server
    pub fn multi_server_toggle(&mut self, server_name: &str) {
        if let Some(pos) = self
            .multi_server_selected
            .iter()
            .position(|s| s == server_name)
        {
            self.multi_server_selected.remove(pos);
        } else {
            self.multi_server_selected.push(server_name.to_string());
        }
    }

    /// Toggle all servers
    pub fn multi_server_toggle_all(&mut self) {
        let all_names: Vec<String> = self
            .tprom_config
            .servers
            .iter()
            .map(|s| s.name.clone())
            .collect();
        if self.multi_server_selected.len() == all_names.len() {
            self.multi_server_selected.clear();
        } else {
            self.multi_server_selected = all_names;
        }
    }

    // ── Tick ─────────────────────────────────────────────────────────────

    pub fn on_tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
        if self.mode == Mode::Splash {
            self.splash_state.spinner_frame = self.spinner_frame % 4;
        }
    }
}

/// A new firing alert shown in the notification toast until dismissed
#[derive(Debug, Clone)]
pub struct AlertToast {
    pub at: chrono::DateTime<chrono::Local>,
    pub server: String,
    pub alertname: String,
    pub severity: String,
    pub summary: String,
}

impl AlertToast {
    fn new(server: &str, alert: &Alert) -> Self {
        let summary = ["summary", "description", "message"]
            .iter()
            .find_map(|k| alert.annotations.get(*k))
            .map(|s| s.lines().next().unwrap_or("").to_string())
            .unwrap_or_default();
        Self {
            at: chrono::Local::now(),
            server: server.to_string(),
            alertname: alert.name().to_string(),
            severity: alert.severity().to_string(),
            summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert(name: &str, severity: &str, instance: &str) -> Alert {
        let labels = [
            ("alertname", name),
            ("severity", severity),
            ("instance", instance),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        Alert {
            labels,
            annotations: HashMap::new(),
            state: "firing".to_string(),
            active_at: String::new(),
            value: String::new(),
        }
    }

    fn multi_app() -> App {
        let mut app = App::new();
        app.tprom_config = TpromConfig {
            alert_sound: false,
            ..TpromConfig::default()
        };
        app.compiled_exclusions.clear();
        app.multi_server_mode_active = true;
        app
    }

    fn fetch(app: &mut App, alerts: Vec<(&str, Vec<Alert>)>, failed: &[&str]) {
        app.multi_server_alerts = alerts
            .into_iter()
            .map(|(s, a)| (s.to_string(), a))
            .collect();
        let failed: Vec<String> = failed.iter().map(|s| s.to_string()).collect();
        app.check_new_firing_alerts(&failed);
    }

    #[test]
    fn first_fetch_is_a_silent_baseline() {
        let mut app = multi_app();
        fetch(
            &mut app,
            vec![("a", vec![alert("Down", "critical", "1")])],
            &[],
        );
        assert!(app.alert_toasts.is_empty());
    }

    #[test]
    fn new_critical_alert_name_notifies_once() {
        let mut app = multi_app();
        fetch(&mut app, vec![("a", vec![])], &[]);
        let firing = vec![
            alert("Down", "critical", "1"),
            alert("Down", "critical", "2"),
        ];
        fetch(&mut app, vec![("a", firing.clone())], &[]);
        assert_eq!(app.alert_toasts.len(), 1);
        assert_eq!(app.alert_toasts[0].alertname, "Down");
        assert_eq!(app.alert_toasts[0].server, "a");

        // Still firing on the next refresh: no new toast
        fetch(&mut app, vec![("a", firing)], &[]);
        assert_eq!(app.alert_toasts.len(), 1);
    }

    #[test]
    fn new_instance_of_firing_alert_is_not_new() {
        let mut app = multi_app();
        fetch(
            &mut app,
            vec![("a", vec![alert("JobFailed", "critical", "job-1")])],
            &[],
        );
        fetch(
            &mut app,
            vec![("a", vec![alert("JobFailed", "critical", "job-2")])],
            &[],
        );
        assert!(app.alert_toasts.is_empty());
    }

    #[test]
    fn non_critical_alerts_are_ignored() {
        let mut app = multi_app();
        fetch(&mut app, vec![("a", vec![])], &[]);
        fetch(
            &mut app,
            vec![("a", vec![alert("HighCpu", "warning", "1")])],
            &[],
        );
        assert!(app.alert_toasts.is_empty());
    }

    #[test]
    fn failed_fetch_does_not_reset_server() {
        let mut app = multi_app();
        let firing = vec![alert("Down", "critical", "1")];
        fetch(&mut app, vec![("a", firing.clone())], &[]);
        fetch(&mut app, vec![("a", vec![])], &["a"]);
        fetch(&mut app, vec![("a", firing)], &[]);
        assert!(app.alert_toasts.is_empty());
    }

    #[test]
    fn server_never_fetched_before_gets_baseline_first() {
        let mut app = multi_app();
        fetch(&mut app, vec![("a", vec![]), ("b", vec![])], &["b"]);
        fetch(
            &mut app,
            vec![("a", vec![]), ("b", vec![alert("Down", "critical", "1")])],
            &[],
        );
        assert!(app.alert_toasts.is_empty());
    }

    #[test]
    fn flapping_alert_is_not_renotified() {
        let mut app = multi_app();
        fetch(&mut app, vec![("a", vec![])], &[]);
        fetch(
            &mut app,
            vec![("a", vec![alert("Down", "critical", "1")])],
            &[],
        );
        fetch(&mut app, vec![("a", vec![])], &[]);
        fetch(
            &mut app,
            vec![("a", vec![alert("Down", "critical", "1")])],
            &[],
        );
        assert_eq!(app.alert_toasts.len(), 1);
    }

    #[test]
    fn excluded_alerts_are_ignored() {
        let mut app = multi_app();
        app.compiled_exclusions = vec![(Regex::new("^Down$").unwrap(), vec![])];
        fetch(&mut app, vec![("a", vec![])], &[]);
        fetch(
            &mut app,
            vec![("a", vec![alert("Down", "critical", "1")])],
            &[],
        );
        assert!(app.alert_toasts.is_empty());
    }

    #[test]
    fn toast_renders_and_lists_new_alerts() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut app = multi_app();
        fetch(
            &mut app,
            vec![("prometheus-production-island05", vec![])],
            &[],
        );
        let mut a = alert("MariaDBDatabaseSize", "critical", "1");
        a.annotations
            .insert("summary".into(), "Database size above 90%".into());
        fetch(
            &mut app,
            vec![("prometheus-production-island05", vec![a])],
            &[],
        );

        let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
        terminal
            .draw(|f| crate::ui::alert_toast::render(f, &app, f.area()))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let text: String = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    + "\n"
            })
            .collect();
        println!("{}", text);
        assert!(text.contains("1 new firing alert"));
        assert!(text.contains("MariaDBDatabaseSize"));
        assert!(text.contains("prometheus-production-island05 · Database size above 90%"));
        assert!(text.contains("Esc dismiss"));
    }
}
