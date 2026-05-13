use crate::cli::SortKey;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f64,
    pub memory_bytes: u64,
    pub memory_pct: f64,
}

pub fn collect_process_metrics(
    sys: &sysinfo::System,
    count: usize,
    sort_key: &SortKey,
) -> Vec<ProcessMetrics> {
    let total_memory = sys.total_memory();
    let mut processes: Vec<ProcessMetrics> = sys
        .processes()
        .values()
        .map(|p| {
            let memory_bytes = p.memory();
            let memory_pct = if total_memory > 0 {
                (memory_bytes as f64 / total_memory as f64) * 100.0
            } else {
                0.0
            };
            ProcessMetrics {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                cpu_usage: f64::from(p.cpu_usage()),
                memory_bytes,
                memory_pct,
            }
        })
        .collect();

    match sort_key {
        SortKey::Cpu => processes.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortKey::Mem => processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
    }

    processes.truncate(count);
    processes
}
