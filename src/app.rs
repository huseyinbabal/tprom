use crate::client::PrometheusClient;
use crate::model::*;
use crate::ui::server_dialog::ServerDialogState;
use crate::ui::splash::SplashState;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mode {
    Splash,
    Normal,
    Describe,
    Confirm,
    Resources,
    ServerDialog,
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
    None,
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

    // Status bar message
    pub status_message: Option<String>,
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
        ];

        let tprom_config = TpromConfig::load();

        Self {
            mode: Mode::Splash,
            active_resource: "targets".to_string(),
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
            status_message: None,
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

        self.tprom_config.add_server(name.clone(), url.clone())?;
        self.current_server = Some(ServerConfig { name, url });
        Ok(())
    }

    pub fn delete_server(&mut self, server_name: &str) -> Result<()> {
        self.tprom_config
            .servers
            .retain(|s| s.name != server_name);
        self.tprom_config.save()?;
        Ok(())
    }

    pub fn current_server_name(&self) -> &str {
        self.current_server
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("No Server")
    }

    // ── Connection ───────────────────────────────────────────────────────

    pub fn connect(&mut self) -> Result<()> {
        if let Some(ref server) = self.current_server {
            self.client = Some(PrometheusClient::new(&server.url)?);
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
            if self.selected_alert_index >= self.alerts.len() {
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
                self.label_values = client
                    .fetch_label_values(label)
                    .await
                    .unwrap_or_default();
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

    pub async fn describe_target(&mut self) -> Result<()> {
        if !self.targets.is_empty() && self.selected_target_index < self.targets.len() {
            let target = self.targets[self.selected_target_index].clone();
            self.describe_data = DescribeData::Target(target);
            self.describe_scroll = 0;
        }
        Ok(())
    }

    pub async fn describe_alert(&mut self) -> Result<()> {
        if !self.alerts.is_empty() && self.selected_alert_index < self.alerts.len() {
            let alert = self.alerts[self.selected_alert_index].clone();
            self.describe_data = DescribeData::Alert(alert);
            self.describe_scroll = 0;
        }
        Ok(())
    }

    pub async fn describe_rule(&mut self) -> Result<()> {
        if !self.flat_rules.is_empty() && self.selected_rule_index < self.flat_rules.len() {
            let (_, rule) = self.flat_rules[self.selected_rule_index].clone();
            self.describe_data = DescribeData::Rule(rule);
            self.describe_scroll = 0;
        }
        Ok(())
    }

    pub async fn describe_metric(&mut self) -> Result<()> {
        if self.filtered_metrics.is_empty() || self.selected_metric_index >= self.filtered_metrics.len() {
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
        if self.filter_text.is_empty() {
            self.alerts.iter().collect()
        } else {
            let filter = self.filter_text.to_lowercase();
            self.alerts
                .iter()
                .filter(|a| a.name().to_lowercase().contains(&filter))
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
                    g.to_lowercase().contains(&filter)
                        || r.name.to_lowercase().contains(&filter)
                })
                .collect()
        }
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
                if !self.targets.is_empty() && self.selected_target_index < self.targets.len() - 1 {
                    self.selected_target_index += 1;
                }
            }
            "alerts" => {
                if !self.alerts.is_empty() && self.selected_alert_index < self.alerts.len() - 1 {
                    self.selected_alert_index += 1;
                }
            }
            "rules" => {
                if !self.flat_rules.is_empty()
                    && self.selected_rule_index < self.flat_rules.len() - 1
                {
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
            "servers" => {
                if !self.tprom_config.servers.is_empty()
                    && self.selected_server_index < self.tprom_config.servers.len() - 1
                {
                    self.selected_server_index += 1;
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
                if self.selected_alert_index > 0 {
                    self.selected_alert_index -= 1;
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
            "servers" => {
                if self.selected_server_index > 0 {
                    self.selected_server_index -= 1;
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
            "alerts" => self.selected_alert_index = 0,
            "rules" => self.selected_rule_index = 0,
            "metrics" => self.selected_metric_index = 0,
            "labels" => {
                if self.label_focus_right {
                    self.selected_label_value_index = 0;
                } else {
                    self.selected_label_index = 0;
                }
            }
            "servers" => self.selected_server_index = 0,
            "tsdb" => self.tsdb_scroll = 0,
            "config" => self.config_scroll = 0,
            "info" => self.info_scroll = 0,
            _ => {}
        }
    }

    pub fn go_to_bottom(&mut self) {
        match self.active_resource.as_str() {
            "targets" => {
                if !self.targets.is_empty() {
                    self.selected_target_index = self.targets.len() - 1;
                }
            }
            "alerts" => {
                if !self.alerts.is_empty() {
                    self.selected_alert_index = self.alerts.len() - 1;
                }
            }
            "rules" => {
                if !self.flat_rules.is_empty() {
                    self.selected_rule_index = self.flat_rules.len() - 1;
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
            "servers" => {
                if !self.tprom_config.servers.is_empty() {
                    self.selected_server_index = self.tprom_config.servers.len() - 1;
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

    // ── Tick ─────────────────────────────────────────────────────────────

    pub fn on_tick(&mut self) {
        if self.mode == Mode::Splash {
            self.splash_state.spinner_frame = (self.splash_state.spinner_frame + 1) % 4;
        }
    }
}
