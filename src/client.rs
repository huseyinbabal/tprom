use crate::model::*;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client for the Prometheus v1 API
pub struct PrometheusClient {
    base_url: String,
    http: reqwest::Client,
}

impl PrometheusClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let url = base_url.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self {
            base_url: url,
            http,
        })
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    // ── Targets ──────────────────────────────────────────────────────────

    pub async fn fetch_targets(&self) -> Result<TargetsData> {
        let resp: ApiResponse<TargetsData> = self
            .http
            .get(self.api_url("/targets"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Alerts ───────────────────────────────────────────────────────────

    pub async fn fetch_alerts(&self) -> Result<AlertsData> {
        let resp: ApiResponse<AlertsData> = self
            .http
            .get(self.api_url("/alerts"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Rules ────────────────────────────────────────────────────────────

    pub async fn fetch_rules(&self) -> Result<RulesData> {
        let resp: ApiResponse<RulesData> = self
            .http
            .get(self.api_url("/rules"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Metric names ─────────────────────────────────────────────────────

    pub async fn fetch_metric_names(&self) -> Result<Vec<String>> {
        let resp: ApiResponse<Vec<String>> = self
            .http
            .get(self.api_url("/label/__name__/values"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Metric metadata ──────────────────────────────────────────────────

    pub async fn fetch_metadata(&self) -> Result<HashMap<String, Vec<MetricMetadata>>> {
        let resp: ApiResponse<HashMap<String, Vec<MetricMetadata>>> = self
            .http
            .get(self.api_url("/metadata"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Labels ───────────────────────────────────────────────────────────

    pub async fn fetch_label_names(&self) -> Result<Vec<String>> {
        let resp: ApiResponse<Vec<String>> = self
            .http
            .get(self.api_url("/labels"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    pub async fn fetch_label_values(&self, label: &str) -> Result<Vec<String>> {
        let resp: ApiResponse<Vec<String>> = self
            .http
            .get(self.api_url(&format!("/label/{}/values", label)))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── TSDB status ──────────────────────────────────────────────────────

    pub async fn fetch_tsdb_status(&self) -> Result<TsdbStatusData> {
        let resp: ApiResponse<TsdbStatusData> = self
            .http
            .get(self.api_url("/status/tsdb"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Config ───────────────────────────────────────────────────────────

    pub async fn fetch_config(&self) -> Result<ConfigData> {
        let resp: ApiResponse<ConfigData> = self
            .http
            .get(self.api_url("/status/config"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Runtime info ─────────────────────────────────────────────────────

    pub async fn fetch_runtime_info(&self) -> Result<RuntimeInfoData> {
        let resp: ApiResponse<RuntimeInfoData> = self
            .http
            .get(self.api_url("/status/runtimeinfo"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Build info ───────────────────────────────────────────────────────

    pub async fn fetch_build_info(&self) -> Result<BuildInfoData> {
        let resp: ApiResponse<BuildInfoData> = self
            .http
            .get(self.api_url("/status/buildinfo"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Flags ────────────────────────────────────────────────────────────

    pub async fn fetch_flags(&self) -> Result<HashMap<String, String>> {
        let resp: ApiResponse<HashMap<String, String>> = self
            .http
            .get(self.api_url("/status/flags"))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Instant query ────────────────────────────────────────────────────

    pub async fn query_instant(&self, expr: &str) -> Result<QueryData> {
        let resp: ApiResponse<QueryData> = self
            .http
            .get(self.api_url("/query"))
            .query(&[("query", expr)])
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.data)
    }

    // ── Health check (uses build info as ping) ───────────────────────────

    pub async fn health_check(&self) -> Result<BuildInfoData> {
        self.fetch_build_info().await
    }
}
