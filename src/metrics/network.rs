#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct NetworkMetrics {
    pub interface: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub total_rx: u64,
    pub total_tx: u64,
}

pub fn collect_network_metrics(networks: &sysinfo::Networks) -> Vec<NetworkMetrics> {
    networks
        .iter()
        .map(|(name, data)| NetworkMetrics {
            interface: name.clone(),
            rx_bytes: data.received(),
            tx_bytes: data.transmitted(),
            total_rx: data.total_received(),
            total_tx: data.total_transmitted(),
        })
        .collect()
}
