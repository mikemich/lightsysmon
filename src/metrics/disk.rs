use sysinfo::Disks;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct DiskMetrics {
    pub name: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub used_pct: f64,
}

pub fn collect_disk_metrics() -> Vec<DiskMetrics> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let used_pct = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            DiskMetrics {
                name: disk.name().to_string_lossy().into_owned(),
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                fs_type: disk.file_system().to_string_lossy().into_owned(),
                total,
                available,
                used,
                used_pct,
            }
        })
        .collect()
}
