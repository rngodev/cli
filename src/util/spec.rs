use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

pub fn load_spec_from_file(spec_path: String) -> Result<Value> {
    let path = Path::new(&spec_path);

    if !path.exists() {
        bail!("Could not find file '{}'", spec_path)
    }

    let file_content = fs::read_to_string(path)?;
    serde_yaml::from_str(&file_content)
        .with_context(|| format!("Failed to parse spec file at {}", path.to_string_lossy()))
}

pub fn load_spec_from_project_directory() -> Result<Value> {
    let rngo_path = Path::new(".rngo");
    let entities_path = rngo_path.join("entities");

    let entity_files = fs::read_dir(entities_path.clone()).with_context(|| {
        format!(
            "Failed to read from entities directory at '{}'",
            entities_path.to_string_lossy()
        )
    })?;

    let mut entities_map = Map::new();

    for entry in entity_files {
        let entry = entry?;
        let path = entry.path();

        let content = fs::read_to_string(&path)?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content).with_context(|| {
            format!("Failed to parse entity file at {}", path.to_string_lossy())
        })?;
        let json_value: serde_json::Value = serde_json::to_value(yaml_value)?;

        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            entities_map.insert(filename.to_string(), json_value);
        }
    }

    if entities_map.len() == 0 {
        bail!(
            "No entities found under {}",
            entities_path.to_string_lossy()
        )
    }

    let systems_map = load_systems_from_project_directory()?;

    // Try to load base spec from .rngo/spec.yml, fallback to default map
    let rngo_spec_path = rngo_path.join("spec.yml");
    let mut spec = if rngo_spec_path.exists() {
        let spec_content = fs::read_to_string(&rngo_spec_path)?;
        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(&spec_content).with_context(|| {
                format!(
                    "Failed to parse spec file at {}",
                    rngo_spec_path.to_string_lossy()
                )
            })?;
        let json_value: serde_json::Value = serde_json::to_value(yaml_value)?;

        if let serde_json::Value::Object(map) = json_value {
            map
        } else {
            bail!(
                "Spec file at {} must contain a YAML object",
                rngo_spec_path.to_string_lossy()
            );
        }
    } else {
        let mut default_spec = Map::new();
        default_spec.insert("seed".into(), 1.into());

        // Set "key" to current directory name
        if let Ok(current_dir) = std::env::current_dir() {
            if let Some(dir_name) = current_dir.file_name().and_then(|s| s.to_str()) {
                default_spec.insert("key".into(), dir_name.into());
            }
        }

        default_spec
    };

    if !systems_map.is_empty() {
        spec.insert("systems".into(), serde_json::Value::Object(systems_map));
    }
    spec.insert("entities".into(), serde_json::Value::Object(entities_map));

    Ok(serde_json::Value::Object(spec))
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

pub fn ensure_spec_output_is_stream(mut spec: Value) -> Value {
    match spec {
        Value::Object(ref mut map) => {
            map.insert("output".into(), "stream".into());
            spec
        }
        _ => spec,
    }
}
