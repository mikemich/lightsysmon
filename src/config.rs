use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub interval: Option<u64>,
    pub show_disk: Option<bool>,
    pub show_network: Option<bool>,
    pub show_processes: Option<bool>,
    pub process_count: Option<usize>,
    pub alert_cpu: Option<f64>,
    pub alert_mem: Option<f64>,
}

pub fn load_config(path: Option<&str>) -> Config {
    let config_path = path.map(|p| p.to_string()).or_else(|| {
        std::env::var("HOME")
            .ok()
            .map(|home| format!("{}/.config/lightsysmon/config.toml", home))
    });

    if let Some(ref p) = config_path {
        if let Ok(content) = std::fs::read_to_string(p) {
            match toml::from_str::<Config>(&content) {
                Ok(cfg) => return cfg,
                Err(e) => eprintln!("Warning: failed to parse config file: {}", e),
            }
        }
    }

    Config::default()
}
