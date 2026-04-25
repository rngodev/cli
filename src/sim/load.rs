use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

use crate::config::Config;

pub fn load_sim_from_file(sim_path: String) -> Result<Value> {
    let path = Path::new(&sim_path);

    if !path.exists() {
        bail!("Could not find file '{}'", sim_path)
    }

    let file_content = fs::read_to_string(path)?;
    serde_yaml::from_str(&file_content)
        .with_context(|| format!("Failed to parse sim file at {}", path.to_string_lossy()))
}

pub fn load_sim_from_project_directory(config: &Config) -> Result<Value> {
    let rngo_path = Path::new(".rngo");
    let effects_path = rngo_path.join("effects");

    let effect_files = fs::read_dir(effects_path.clone()).with_context(|| {
        format!(
            "Failed to read from effects directory at '{}'",
            effects_path.to_string_lossy()
        )
    })?;

    let mut effects_map = Map::new();

    for entry in effect_files {
        let entry = entry?;
        let path = entry.path();

        let content = fs::read_to_string(&path)?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content).with_context(|| {
            format!("Failed to parse effect file at {}", path.to_string_lossy())
        })?;
        let mut json_value: serde_json::Value = serde_json::to_value(yaml_value)?;

        if let Some(obj) = json_value.as_object_mut() {
            obj.entry("type").or_insert_with(|| "state.create".into());
        }

        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            effects_map.insert(filename.to_string(), json_value);
        }
    }

    if effects_map.is_empty() {
        bail!("No effects found under {}", effects_path.to_string_lossy())
    }

    let systems_map = load_systems_from_project_directory()?;

    let mut sim = Map::new();
    sim.insert("seed".into(), config.seed.into());

    if let Some(key) = &config.key {
        sim.insert("key".into(), key.clone().into());
    } else {
        let dir_name = std::env::current_dir()
            .ok()
            .and_then(|dir| {
                dir.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| anyhow!("Failed to get current directory"))?;

        sim.insert("key".into(), dir_name.into());
    }

    if let Some(start) = &config.start {
        sim.insert("start".into(), start.clone().into());
    }

    if let Some(end) = &config.end {
        sim.insert("end".into(), end.clone().into());
    }

    if !systems_map.is_empty() {
        sim.insert("systems".into(), serde_json::Value::Object(systems_map));
    }
    sim.insert("effects".into(), serde_json::Value::Object(effects_map));

    Ok(serde_json::Value::Object(sim))
}

pub fn load_systems_from_project_directory() -> Result<Map<String, Value>> {
    let rngo_path = Path::new(".rngo");
    let systems_path = rngo_path.join("systems");
    let mut systems_map = Map::new();

    if systems_path.is_dir() {
        let system_files = fs::read_dir(systems_path.clone()).with_context(|| {
            format!(
                "Failed to read from systems directory at '{}'",
                systems_path.to_string_lossy()
            )
        })?;

        for system in system_files {
            let system = system?;
            let path = system.path();

            let content = fs::read_to_string(&path)?;
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse file at {}", path.to_string_lossy()))?;
            let json_value: serde_json::Value = serde_json::to_value(yaml_value)?;

            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                systems_map.insert(filename.to_string(), json_value);
            }
        }
    }

    Ok(systems_map)
}
