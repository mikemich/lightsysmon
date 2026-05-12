#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct CpuMetrics {
    pub global_usage: f64,
    pub per_core: Vec<f64>,
    pub cpu_count: usize,
    pub brand: String,
    pub frequency_mhz: u64,
}

pub fn collect_cpu_metrics(sys: &sysinfo::System) -> CpuMetrics {
    let cpus = sys.cpus();
    let per_core: Vec<f64> = cpus.iter().map(|c| f64::from(c.cpu_usage())).collect();
    let cpu_count = per_core.len();
    let brand = cpus.first().map_or_else(String::new, |c| c.brand().to_string());
    let frequency_mhz = cpus.first().map_or(0, |c| c.frequency());

    CpuMetrics {
        global_usage: f64::from(sys.global_cpu_usage()),
        per_core,
        cpu_count,
        brand,
        frequency_mhz,
    }
}
