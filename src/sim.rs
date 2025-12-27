mod problem;
mod sink;

use crate::sim::problem::Problem;
use crate::util::model::{Entity, Simulation, SimulationRun, SimulationRunData, System};
use anyhow::{Context, Result, anyhow};
use futures::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sink::SimulationSink;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum EventData {
    Create {
        id: u64,
        entity: String,
        offset: i64,
        value: Value,
    },
    Error {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        entity: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<Vec<String>>,
        message: String,
    },
}

pub async fn sim(spec: Option<String>, stdout: bool) -> Result<()> {
    let config = crate::util::config::get_config()?;
    let api_key = config
        .api_key
        .ok_or_else(|| anyhow!("Could not find API key"))?;

    let client = reqwest::Client::new();

    let (key, spec) = {
        let spec = if let Some(spec) = spec {
            crate::util::spec::load_spec_from_file(spec)?
        } else {
            crate::util::spec::load_spec_from_project_directory()?
        };

        let mut spec = crate::util::spec::ensure_spec_output_is_stream(spec);
        let key = crate::util::spec::get_spec_key(&mut spec);
        (key, spec)
    };

    let simulation = {
        let push_simulation_response = match key {
            Some(key) => {
                client
                    .put(format!(
                        "{api_url}/simulations/{key}",
                        api_url = config.api_url,
                        key = key
                    ))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&spec)
                    .send()
                    .await?
            }
            None => {
                client
                    .post(format!("{api_url}/simulations", api_url = config.api_url))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&spec)
                    .send()
                    .await?
            }
        };

        if !push_simulation_response.status().is_success() {
            let status = push_simulation_response.status().clone();
            let problem = push_simulation_response.json::<Problem>().await?;

            return Err(problem).with_context(|| match status {
                StatusCode::UNPROCESSABLE_ENTITY => "Validation error",
                _ => "API error",
            })?;
        }

        push_simulation_response.json::<Simulation>().await?
    };

    let simulation_run = {
        let response = client
            .post(format!(
                "{api_url}/simulationRuns",
                api_url = config.api_url
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "simulation": simulation.key,
                "output": "stream",
            }))
            .send()
            .await?;

        response.json::<SimulationRun>().await?
    };

    let simulation_run_directory = format!(".rngo/runs/{}", simulation_run.id);
    let simulation_run_directory = Path::new(&simulation_run_directory);

    if !stdout {
        fs::create_dir_all(simulation_run_directory)?;
    }

    let simulation_run_data = {
        let entities_response = client
            .get(format!(
                "{api_url}/simulationRuns/{id}/entities",
                api_url = config.api_url,
                id = simulation_run.id
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        let entities = entities_response.json::<Vec<Entity>>().await?;

        let systems_response = client
            .get(format!(
                "{api_url}/simulationRuns/{id}/systems",
                api_url = config.api_url,
                id = simulation_run.id
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        let systems = systems_response.json::<Vec<System>>().await?;

        SimulationRunData {
            id: simulation_run.id.clone(),
            entities,
            systems,
        }
    };

    let mut simulation_sink = if stdout {
        SimulationSink::stream()
    } else {
        SimulationSink::try_from(simulation_run_data.clone())?
    };

    let stream_url = format!(
        "{api_url}/simulationRuns/{id}/stream",
        api_url = config.api_url,
        id = simulation_run.id
    );

    // Track the last event ID for seamless reconnection
    let mut last_event_id: Option<u64> = None;

    // Loop to handle reconnection
    loop {
        let mut request = client
            .get(&stream_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/x-ndjson");

        // Add lastEventId query parameter if we have one
        if let Some(event_id) = last_event_id {
            request = request.query(&[("lastEventId", event_id.to_string())]);
        }

        let response = request.send().await?;

        let status = response.status();

        // If we get 204 No Content, the simulation is complete
        if status == StatusCode::NO_CONTENT {
            break;
        }

        if !status.is_success() {
            let problem = response.json::<Problem>().await?;
            return Err(problem).with_context(|| "API error while streaming")?;
        }

        // Process the NDJSON stream
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = match chunk_result {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Stream error: {}, reconnecting...", e);
                    break; // Break inner loop to reconnect
                }
            };

            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if !line.is_empty() {
                    match serde_json::from_str::<EventData>(&line) {
                        Ok(event_data) => {
                            // Track the last event ID for reconnection
                            last_event_id = Some(match &event_data {
                                EventData::Create { id, .. } => *id,
                                EventData::Error { id, .. } => *id,
                            });
                            simulation_sink.write_event(event_data);
                        }
                        Err(e) => eprintln!("Failed to parse NDJSON line: {} - Error: {}", line, e),
                    }
                }
            }
        }

        // If we reach here, the connection ended without 204, so reconnect
    }

    if !stdout {
        let entities_map: serde_json::Map<String, Value> = simulation_run_data
            .entities
            .into_iter()
            .map(|entity| {
                let key = entity.key.clone();
                let mut value = serde_json::to_value(entity).unwrap();
                if let Some(obj) = value.as_object_mut() {
                    obj.remove("key");
                }
                (key, value)
            })
            .collect();

        let systems_map: serde_json::Map<String, Value> = simulation_run_data
            .systems
            .into_iter()
            .map(|system| {
                let key = system.key.clone();
                let mut value = serde_json::to_value(system).unwrap();
                if let Some(obj) = value.as_object_mut() {
                    obj.remove("key");
                }
                (key, value)
            })
            .collect();

        let mut spec = serde_json::Map::new();
        spec.insert("seed".to_string(), json!(simulation.seed));
        spec.insert("parent".to_string(), json!(simulation.parent));
        spec.insert("entities".to_string(), json!(entities_map));
        spec.insert("systems".to_string(), json!(systems_map));

        let spec_path = simulation_run_directory.join("spec.yml");
        fs::write(spec_path, serde_json::to_string_pretty(&spec)?)?;

        println!("Created and ran simulation");
        println!("  fs:  .rngo/runs/{}", simulation_run.id);
        println!("  sim: https://rngo.dev/simulations/{}", simulation.key);
        println!(
            "  run: https://rngo.dev/simulationRuns/{}",
            simulation_run.id
        );
    }

    Ok(())
}
