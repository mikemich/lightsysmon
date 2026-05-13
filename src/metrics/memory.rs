#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct MemoryMetrics {
    pub used: u64,
    pub total: u64,
    pub usage_pct: f64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub swap_pct: f64,
}

pub fn collect_memory_metrics(sys: &sysinfo::System) -> MemoryMetrics {
    let used = sys.used_memory();
    let total = sys.total_memory();
    let usage_pct = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let swap_used = sys.used_swap();
    let swap_total = sys.total_swap();
    let swap_pct = if swap_total > 0 {
        (swap_used as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };

    MemoryMetrics {
        used,
        total,
        usage_pct,
        swap_used,
        swap_total,
        swap_pct,
    }
}
