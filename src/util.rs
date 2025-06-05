use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    pub api_key: Option<String>,
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn default_api_url() -> String {
    "https://api.rngo.dev".into()
}

pub fn get_config() -> Result<Config> {
    let config = config::Config::builder()
        .add_source(config::File::from(user_config_file_path()?).required(false))
        .add_source(config::Environment::with_prefix("RNGO"))
        .build()?;

    config
        .try_deserialize::<Config>()
        .with_context(|| "Failed to deserialize config")
}

pub fn set_config(config: Config) -> Result<()> {
    let mut file_base_path = user_config_file_path()?;
    file_base_path.set_extension("yml");

    if let Some(parent) = file_base_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(file_base_path).with_context(|| "Failed to open config file")?;
    let yaml = serde_yaml::to_string(&config).with_context(|| "Failed to serialize config")?;
    file.write_all(yaml.as_bytes())
        .with_context(|| "Failed to write config")
}

fn user_config_file_path() -> Result<PathBuf> {
    let mut config_path = ProjectDirs::from("dev", "rngo", "cli")
        .map(|project_dirs| project_dirs.config_dir().to_path_buf())
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;

    config_path.push("config");
    Ok(config_path)
}
