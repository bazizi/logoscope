const CONFIGS_PATH: &str = "logoscope";

pub fn get_config_dir_path() -> std::path::PathBuf {
    [&std::env::var("LOCALAPPDATA").unwrap(), CONFIGS_PATH]
        .iter()
        .collect()
}
