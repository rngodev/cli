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
    let spec = if let Some(spec) = spec {
        crate::util::spec::load_spec_from_file(spec)?
    } else {
        crate::util::spec::load_spec_from_project_directory()?
    };
    let mut spec = crate::util::spec::ensure_spec_output_is_stream(spec);
    let key = crate::util::spec::get_spec_key(&mut spec);

    let config = crate::util::config::get_config()?;
    let api_key = config
        .api_key
        .ok_or_else(|| anyhow!("Could not find API key"))?;

    let client = reqwest::Client::new();

    let response = match key {
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

    if !response.status().is_success() {
        let status = response.status().clone();
        let problem = response.json::<Problem>().await?;

        return Err(problem).with_context(|| match status {
            StatusCode::UNPROCESSABLE_ENTITY => "Validation error",
            _ => "API error",
        })?;
    }

    let simulation = response.json::<Simulation>().await?;

    let simulation_run_response = client
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

    let simulation_run = simulation_run_response.json::<SimulationRun>().await?;

    let simulation_run_directory = format!(".rngo/runs/{}", simulation_run.id);
    let simulation_run_directory = Path::new(&simulation_run_directory);

    let simulation_run_entities_response = client
        .get(format!(
            "{api_url}/simulationRuns/{id}/entities",
            api_url = config.api_url,
            id = simulation_run.id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let simulation_run_entities = simulation_run_entities_response
        .json::<Vec<Entity>>()
        .await?;

    let simulation_run_systems_response = client
        .get(format!(
            "{api_url}/simulationRuns/{id}/systems",
            api_url = config.api_url,
            id = simulation_run.id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let simulation_run_systems = simulation_run_systems_response
        .json::<Vec<System>>()
        .await?;

    let simulation_run_data = SimulationRunData {
        id: simulation_run.id.clone(),
        entities: simulation_run_entities,
        systems: simulation_run_systems,
    };

    if !stdout {
        fs::create_dir_all(simulation_run_directory)?;
    }

    let mut simulation_sink = if stdout {
        SimulationSink::stream()
    } else {
        SimulationSink::try_from(simulation_run_data)?
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
        let get_simulation_run_response = client
            .get(format!(
                "{api_url}/simulationRuns",
                api_url = config.api_url
            ))
            .query(&[("id", simulation_run.id.as_str())])
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        let simulation_run_json = get_simulation_run_response.json::<Value>().await?;

        let simulation_json = simulation_run_json
            .as_array()
            .context("Expected array response from /simulationRuns")?
            .first();

        let simulation_metadata_directory = simulation_run_directory.join("metadata");
        let spec_path = simulation_metadata_directory.join("simulation.json");
        fs::create_dir_all(simulation_metadata_directory)?;
        fs::write(spec_path, serde_json::to_string_pretty(&simulation_json)?)?;

        println!("Created and ran simulation");
        println!("See https://rngo.dev/simulationRuns/{}", simulation_run.id);
    }

    Ok(())
}
