use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── Server / Config persistence ──────────────────────────────────────────────

/// A saved Prometheus server connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub url: String,
}

/// Root config file stored in XDG config dir
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TpromConfig {
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

impl TpromConfig {
    /// Get the config file path (~/.config/tprom/config.yaml)
    pub fn config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("tprom").join("config.yaml")
        } else if let Some(home) = dirs::home_dir() {
            home.join(".config").join("tprom").join("config.yaml")
        } else {
            PathBuf::from("config.yaml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_yaml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn add_server(&mut self, name: String, url: String) -> anyhow::Result<()> {
        self.servers.push(ServerConfig { name, url });
        self.save()
    }
}

// ── Prometheus API response wrappers ─────────────────────────────────────────

/// Generic Prometheus API response envelope
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: T,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(rename = "errorType")]
    #[serde(default)]
    pub error_type: Option<String>,
}

// ── Targets ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TargetsData {
    #[serde(rename = "activeTargets")]
    pub active_targets: Vec<ActiveTarget>,
    #[serde(rename = "droppedTargets")]
    #[serde(default)]
    pub dropped_targets: Vec<DroppedTarget>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActiveTarget {
    #[serde(rename = "discoveredLabels")]
    pub discovered_labels: HashMap<String, String>,
    pub labels: HashMap<String, String>,
    #[serde(rename = "scrapePool")]
    pub scrape_pool: String,
    #[serde(rename = "scrapeUrl")]
    pub scrape_url: String,
    #[serde(rename = "globalUrl")]
    #[serde(default)]
    pub global_url: String,
    #[serde(rename = "lastError")]
    pub last_error: String,
    #[serde(rename = "lastScrape")]
    pub last_scrape: String,
    #[serde(rename = "lastScrapeDuration")]
    pub last_scrape_duration: f64,
    pub health: String, // "up", "down", "unknown"
}

impl ActiveTarget {
    pub fn job(&self) -> &str {
        self.labels
            .get("job")
            .map(|s| s.as_str())
            .unwrap_or("unknown")
    }

    pub fn instance(&self) -> &str {
        self.labels
            .get("instance")
            .map(|s| s.as_str())
            .unwrap_or("unknown")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DroppedTarget {
    #[serde(rename = "discoveredLabels")]
    pub discovered_labels: HashMap<String, String>,
}

// ── Alerts ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AlertsData {
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Alert {
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub state: String, // "firing", "pending", "inactive"
    #[serde(rename = "activeAt")]
    #[serde(default)]
    pub active_at: String,
    pub value: String,
}

impl Alert {
    pub fn name(&self) -> &str {
        self.labels
            .get("alertname")
            .map(|s| s.as_str())
            .unwrap_or("unknown")
    }
}

// ── Rules ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct RulesData {
    pub groups: Vec<RuleGroup>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleGroup {
    pub name: String,
    pub file: String,
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub interval: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub name: String,
    pub query: String,
    #[serde(rename = "type")]
    pub rule_type: String, // "recording" or "alerting"
    #[serde(default)]
    pub duration: f64,
    pub health: String, // "ok", "err", "unknown"
    #[serde(rename = "lastEvaluation")]
    #[serde(default)]
    pub last_evaluation: String,
    #[serde(rename = "evaluationTime")]
    #[serde(default)]
    pub evaluation_time: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    #[serde(default)]
    pub alerts: Vec<Alert>,
}

// ── Metrics / Metadata ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct MetricMetadata {
    #[serde(rename = "type")]
    pub metric_type: String, // "counter", "gauge", "histogram", "summary", "unknown"
    pub help: String,
    pub unit: String,
}

/// A metric name with optional resolved metadata
#[derive(Debug, Clone)]
pub struct MetricInfo {
    pub name: String,
    pub metric_type: String,
    pub help: String,
}

// ── Labels ───────────────────────────────────────────────────────────────────

// Labels are returned as Vec<String> from the API, no special struct needed.

// ── TSDB Status ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TsdbStatusData {
    #[serde(rename = "headStats")]
    pub head_stats: HeadStats,
    #[serde(rename = "seriesCountByMetricName")]
    pub series_count_by_metric_name: Vec<TsdbStat>,
    #[serde(rename = "labelValueCountByLabelName")]
    pub label_value_count_by_label_name: Vec<TsdbStat>,
    #[serde(rename = "memoryInBytesByLabelName")]
    pub memory_in_bytes_by_label_name: Vec<TsdbStat>,
    #[serde(rename = "seriesCountByLabelValuePair")]
    pub series_count_by_label_value_pair: Vec<TsdbStat>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeadStats {
    #[serde(rename = "numSeries")]
    pub num_series: u64,
    #[serde(rename = "numLabelPairs")]
    #[serde(default)]
    pub num_label_pairs: u64,
    #[serde(rename = "chunkCount")]
    pub chunk_count: u64,
    #[serde(rename = "minTime")]
    pub min_time: i64,
    #[serde(rename = "maxTime")]
    pub max_time: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TsdbStat {
    pub name: String,
    pub value: u64,
}

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigData {
    pub yaml: String,
}

// ── Runtime Info ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeInfoData {
    #[serde(rename = "startTime")]
    #[serde(default)]
    pub start_time: String,
    #[serde(rename = "CWD")]
    #[serde(default)]
    pub cwd: String,
    #[serde(rename = "reloadConfigSuccess")]
    #[serde(default)]
    pub reload_config_success: bool,
    #[serde(rename = "lastConfigTime")]
    #[serde(default)]
    pub last_config_time: String,
    #[serde(rename = "corruptionCount")]
    #[serde(default)]
    pub corruption_count: u64,
    #[serde(rename = "goroutineCount")]
    #[serde(default)]
    pub goroutine_count: u64,
    #[serde(rename = "GOMAXPROCS")]
    #[serde(default)]
    pub gomaxprocs: u64,
    #[serde(rename = "GOGC")]
    #[serde(default)]
    pub gogc: String,
    #[serde(rename = "GODEBUG")]
    #[serde(default)]
    pub godebug: String,
    #[serde(rename = "storageRetention")]
    #[serde(default)]
    pub storage_retention: String,
}

// ── Build Info ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct BuildInfoData {
    pub version: String,
    pub revision: String,
    pub branch: String,
    #[serde(rename = "buildUser")]
    #[serde(default)]
    pub build_user: String,
    #[serde(rename = "buildDate")]
    #[serde(default)]
    pub build_date: String,
    #[serde(rename = "goVersion")]
    pub go_version: String,
}

// ── Flags ────────────────────────────────────────────────────────────────────

// Flags are returned as HashMap<String, String> from the API.

// ── Query results ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct QueryData {
    #[serde(rename = "resultType")]
    pub result_type: String, // "vector", "matrix", "scalar", "string"
    pub result: Vec<QueryResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryResult {
    pub metric: HashMap<String, String>,
    pub value: Option<(f64, String)>,       // instant query
    pub values: Option<Vec<(f64, String)>>, // range query
}

impl QueryResult {
    /// Format the metric labels as a string like {job="foo", instance="bar"}
    pub fn label_string(&self) -> String {
        if self.metric.is_empty() {
            return "{}".to_string();
        }
        let pairs: Vec<String> = self
            .metric
            .iter()
            .filter(|(k, _)| k.as_str() != "__name__")
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect();
        let name = self.metric.get("__name__").cloned().unwrap_or_default();
        if pairs.is_empty() {
            name
        } else {
            format!("{}{{{}}}", name, pairs.join(", "))
        }
    }

    /// Get the value from an instant query result
    pub fn instant_value(&self) -> &str {
        self.value.as_ref().map(|(_, v)| v.as_str()).unwrap_or("")
    }
}
