use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    api_key: Option<String>,
}

pub fn get_config() -> Result<Config> {
    let config = config::Config::builder()
        .add_source(config::File::from(user_config_file_path()?).required(false))
        .build()?;

    config
        .try_deserialize::<Config>()
        .with_context(|| "Failed to parse config")
}

fn user_config_file_path() -> Result<PathBuf> {
    let mut config_path = ProjectDirs::from("dev", "rngo", "cli")
        .map(|project_dirs| project_dirs.config_dir().to_path_buf())
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;

    config_path.push("config");
    Ok(config_path)
}
