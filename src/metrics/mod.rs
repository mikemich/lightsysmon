pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod process;

pub use cpu::{collect_cpu_metrics, CpuMetrics};
pub use disk::{collect_disk_metrics, DiskMetrics};
pub use memory::{collect_memory_metrics, MemoryMetrics};
pub use network::{collect_network_metrics, NetworkMetrics};
pub use process::{collect_process_metrics, ProcessMetrics};

use crate::cli::SortKey;

#[derive(Debug, serde::Serialize, Clone)]
pub struct AllMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub disk: Vec<DiskMetrics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub network: Vec<NetworkMetrics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<ProcessMetrics>,
}

pub struct CollectArgs<'a> {
    pub disk: bool,
    pub network: bool,
    pub processes: bool,
    pub process_count: usize,
    pub sort_key: &'a SortKey,
    pub include_timestamp: bool,
}

pub struct Collector {
    pub sys: sysinfo::System,
    pub networks: sysinfo::Networks,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            sys: sysinfo::System::new_all(),
            networks: sysinfo::Networks::new_with_refreshed_list(),
        }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        self.networks.refresh();
    }

    pub fn collect(&self, args: &CollectArgs) -> AllMetrics {
        let timestamp = if args.include_timestamp {
            Some(chrono::Local::now().to_rfc3339())
        } else {
            None
        };

        AllMetrics {
            timestamp,
            cpu: collect_cpu_metrics(&self.sys),
            memory: collect_memory_metrics(&self.sys),
            disk: if args.disk {
                collect_disk_metrics()
            } else {
                vec![]
            },
            network: if args.network {
                collect_network_metrics(&self.networks)
            } else {
                vec![]
            },
            processes: if args.processes {
                collect_process_metrics(&self.sys, args.process_count, args.sort_key)
            } else {
                vec![]
            },
        }
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}
