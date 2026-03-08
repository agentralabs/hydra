use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hydra_db::HydraDb;
use hydra_server::{build_router, AppState};

/// Stress test server harness with metrics collection
pub struct StressServer {
    pub addr: std::net::SocketAddr,
    server_handle: tokio::task::JoinHandle<()>,
    pub metrics: Arc<StressMetrics>,
}

/// Metrics collected during stress tests
pub struct StressMetrics {
    pub requests_sent: AtomicU64,
    pub requests_succeeded: AtomicU64,
    pub requests_failed: AtomicU64,
    pub max_latency_us: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub concurrent_peak: AtomicUsize,
}

impl StressMetrics {
    pub fn new() -> Self {
        Self {
            requests_sent: AtomicU64::new(0),
            requests_succeeded: AtomicU64::new(0),
            requests_failed: AtomicU64::new(0),
            max_latency_us: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            concurrent_peak: AtomicUsize::new(0),
        }
    }

    pub fn record_request(&self, success: bool, latency: Duration) {
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        if success {
            self.requests_succeeded.fetch_add(1, Ordering::Relaxed);
        } else {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);
        }
        let us = latency.as_micros() as u64;
        self.total_latency_us.fetch_add(us, Ordering::Relaxed);
        self.max_latency_us.fetch_max(us, Ordering::Relaxed);
    }

    pub fn success_rate(&self) -> f64 {
        let sent = self.requests_sent.load(Ordering::Relaxed);
        if sent == 0 {
            return 1.0;
        }
        self.requests_succeeded.load(Ordering::Relaxed) as f64 / sent as f64
    }

    pub fn avg_latency_ms(&self) -> f64 {
        let sent = self.requests_sent.load(Ordering::Relaxed);
        if sent == 0 {
            return 0.0;
        }
        (self.total_latency_us.load(Ordering::Relaxed) as f64 / sent as f64) / 1000.0
    }

    pub fn max_latency_ms(&self) -> f64 {
        self.max_latency_us.load(Ordering::Relaxed) as f64 / 1000.0
    }
}

impl Default for StressMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl StressServer {
    pub async fn start() -> Self {
        let db = HydraDb::in_memory().unwrap();
        let state = AppState::new(db, false, None);
        let app = build_router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        Self {
            addr,
            server_handle,
            metrics: Arc::new(StressMetrics::new()),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    /// Send a single hydra.run request and record metrics
    pub async fn timed_run(&self, client: &reqwest::Client, intent: &str) -> bool {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "stress",
            "method": "hydra.run",
            "params": {"intent": intent},
        });
        let start = Instant::now();
        let result = client.post(self.url("/rpc")).json(&body).send().await;
        let latency = start.elapsed();
        let success = result.is_ok() && result.unwrap().status().is_success();
        self.metrics.record_request(success, latency);
        success
    }

    /// Send a health check and record metrics
    pub async fn timed_health(&self, client: &reqwest::Client) -> bool {
        let start = Instant::now();
        let result = client.get(self.url("/health")).send().await;
        let latency = start.elapsed();
        let success = result.is_ok() && result.unwrap().status().is_success();
        self.metrics.record_request(success, latency);
        success
    }
}

impl Drop for StressServer {
    fn drop(&mut self) {
        self.server_handle.abort();
    }
}
