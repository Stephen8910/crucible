use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub uptime: u64,
    pub timestamp: DateTime<Utc>,
}

pub struct MetricsExporter {
    current_metrics: Arc<RwLock<SystemMetrics>>,
}

impl MetricsExporter {
    pub fn new() -> Self {
        Self {
            current_metrics: Arc::new(RwLock::new(SystemMetrics {
                timestamp: Utc::now(),
                ..Default::default()
            })),
        }
    }

    pub async fn update_metrics(&self, cpu: f64, mem: u64, uptime: u64) {
        let mut metrics = self.current_metrics.write().await;
        metrics.cpu_usage = cpu;
        metrics.memory_usage = mem;
        metrics.uptime = uptime;
        metrics.timestamp = Utc::now();
        info!(metrics = ?*metrics, "Updated system metrics");
    }

    pub async fn get_metrics(&self) -> SystemMetrics {
        self.current_metrics.read().await.clone()
    }

    pub async fn run_collector(exporter: Arc<Self>) {
        info!("Starting system metrics collector worker");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        let start_time = Utc::now();

        loop {
            interval.tick().await;
            let uptime = (Utc::now() - start_time).num_seconds() as u64;
            // Simulated metrics collection
            exporter.update_metrics(12.5, 1024 * 1024 * 512, uptime).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collection() {
        let exporter = MetricsExporter::new();
        exporter.update_metrics(25.0, 1024, 60).await;
        
        let metrics = exporter.get_metrics().await;
        assert_eq!(metrics.cpu_usage, 25.0);
        assert_eq!(metrics.memory_usage, 1024);
        assert_eq!(metrics.uptime, 60);
    }
}
