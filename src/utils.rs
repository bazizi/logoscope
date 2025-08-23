const CONFIGS_PATH: &str = "logoscope";

pub fn get_config_dir_path() -> std::path::PathBuf {
    [
        &std::env::var("LOCALAPPDATA").unwrap_or(std::env::var("HOME").unwrap_or("".to_string())),
        CONFIGS_PATH,
    ]
    .iter()
    .collect()
}
