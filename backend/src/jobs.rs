use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMonitorJob {
    pub tx_hash: String,
}

/// Handler for monitoring Stellar transactions.
/// Returning () since Apalis 0.6 handlers can return ().
pub async fn monitor_transaction(
    job: TransactionMonitorJob,
) {
    info!("Monitoring Stellar transaction: {}", job.tx_hash);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
}
