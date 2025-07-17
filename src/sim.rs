mod simulation_sink;

use anyhow::{Context, Result, anyhow, bail};
use eventsource_client::{Client, SSE};
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use simulation_sink::SimulationSink;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum OutputType {
    Sql,
    Json,
}

#[derive(Clone, Debug, Deserialize)]
struct Output {
    #[serde(rename = "type")]
    otype: OutputType,
}

#[derive(Clone, Debug, Deserialize)]
struct SystemImport {
    command: String,
}

#[derive(Clone, Debug, Deserialize)]
struct System {
    output: Output,
    import: SystemImport,
}

#[derive(Clone, Debug, Deserialize)]
struct EntitySystem {
    #[serde(rename = "type")]
    stype: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Entity {
    output: Option<Output>,
    system: Option<EntitySystem>,
}

#[derive(Clone, Debug, Deserialize)]
struct Simulation {
    id: String,
    entities: HashMap<String, Entity>,
    systems: HashMap<String, System>,
}

#[derive(Debug, Deserialize, Serialize)]
struct EventData {
    entity: String,
    offset: i64,
    value: Value,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PathPart {
    Index(i64),
    Field(String),
}

#[derive(Debug, Deserialize)]
struct ProblemIssue {
    message: String,
    path: Option<Vec<PathPart>>,
}

impl fmt::Display for ProblemIssue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = match &self.path {
            Some(path) => {
                let mut path_str = "".to_string();

                for path_part in path {
                    match path_part {
                        PathPart::Index(i) => path_str += &format!("[{}]", i),
                        PathPart::Field(s) if path_str.len() > 0 => path_str += &format!(".{}", s),
                        PathPart::Field(s) => path_str = s.into(),
                    }
                }

                &format!("{path}: {message}", path = path_str, message = self.message)
            }
            None => &self.message,
        };

        write!(f, "{}", str)
    }
}

#[derive(Debug, Deserialize)]
struct Problem {
    title: String,
    issues: Vec<ProblemIssue>,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.issues.len() > 0 {
            let issues = self
                .issues
                .iter()
                .map(|item| format!("  {}", item.to_string()))
                .collect::<Vec<_>>()
                .join("\n");

            write!(f, "{title}\n{issues}", title = self.title, issues = issues)
        } else {
            write!(f, "{}", self.title)
        }
    }
}

impl std::error::Error for Problem {}

pub async fn sim(spec_path: Option<String>, stream: bool) -> Result<()> {
    let spec = if let Some(spec_path) = spec_path {
        load_spec_from_file(spec_path)?
    } else {
        load_spec_from_project_directory()?
    };

    let config = crate::util::get_config()?;
    let api_key = config
        .api_key
        .ok_or_else(|| anyhow!("Could not find API key"))?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{api_url}/simulations", api_url = config.api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&spec)
        .send()
        .await?;

    if response.status() != StatusCode::CREATED {
        let status = response.status().clone();
        let problem = response.json::<Problem>().await?;

        return Err(problem).with_context(|| match status {
            StatusCode::UNPROCESSABLE_ENTITY => "Validation error",
            _ => "API error",
        })?;
    }

    let simulation = response.json::<Simulation>().await?;

    let simulation_directory = format!(".rngo/simulations/{}", simulation.id);
    let simulation_directory = Path::new(&simulation_directory);

    if !stream {
        fs::create_dir_all(simulation_directory)?;
    }

    let sse_client = eventsource_client::ClientBuilder::for_url(&format!(
        "{api_url}/simulations/{id}/stream",
        api_url = config.api_url,
        id = simulation.id
    ))?
    .header("Authorization", &format!("Bearer {}", api_key))?
    .build();

    let mut sse_stream = sse_client.stream();

    let mut simulation_sink = SimulationSink::try_from(simulation.clone())?;

    while let Ok(Some(sse)) = sse_stream.try_next().await {
        match sse {
            SSE::Event(event) => match serde_json::from_str::<EventData>(&event.data) {
                Ok(event_data) => {
                    if stream {
                        println!("{}", serde_json::to_string(&event_data)?);
                    } else {
                        simulation_sink.write_event(event_data);
                    }
                }
                Err(_) => eprintln!("Failed to parse SSE data: {}", event.data),
            },
            SSE::Connected(_) => (),
            SSE::Comment(_) => (),
        }
    }

    if !stream {
        let response = client
            .get(format!(
                "{api_url}/simulations/{id}",
                api_url = config.api_url,
                id = simulation.id
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        let simulation = response.json::<Value>().await?;

        let simulation_metadata_directory = simulation_directory.join("metadata");
        let spec_path = simulation_metadata_directory.join("simulation.json");
        fs::create_dir_all(simulation_metadata_directory)?;
        fs::write(spec_path, serde_json::to_string_pretty(&simulation)?)?;

        println!(
            "Created simulation and drained to {}",
            simulation_directory.display()
        );
    }

    Ok(())
}

fn load_spec_from_file(spec_path: String) -> Result<Value> {
    let path = Path::new(&spec_path);

    if !path.exists() {
        bail!("Could not find file '{}'", spec_path)
    }

    let file_content = fs::read_to_string(path)?;
    serde_yaml::from_str(&file_content)
        .with_context(|| format!("Failed to parse spec file at {}", path.to_string_lossy()))
}

fn load_spec_from_project_directory() -> Result<Value> {
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

    let mut spec = Map::new();
    spec.insert("seed".into(), 1.into());
    if !systems_map.is_empty() {
        spec.insert("systems".into(), serde_json::Value::Object(systems_map));
    }
    spec.insert("entities".into(), serde_json::Value::Object(entities_map));

    Ok(serde_json::Value::Object(spec))
}
