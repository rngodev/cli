mod model;
mod problem;
mod sink;
mod spec;

use crate::sim::model::Simulation;
use crate::sim::problem::Problem;
use anyhow::{Context, Result, anyhow};
use eventsource_client::{Client, SSE};
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sink::SimulationSink;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum EventData {
    Create {
        entity: String,
        offset: i64,
        value: Value,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        entity: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<Vec<String>>,
        message: String,
    },
}

pub async fn sim(spec_path: Option<String>, stream: bool) -> Result<()> {
    let spec = if let Some(spec_path) = spec_path {
        spec::load_spec_from_file(spec_path)?
    } else {
        spec::load_spec_from_project_directory()?
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

    let simulation_id = &simulation.id;
    let simulation_directory = format!(".rngo/simulations/{}", simulation_id);
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

    let mut simulation_sink = if stream {
        SimulationSink::stream()
    } else {
        SimulationSink::try_from(simulation.clone())?
    };

    while let Ok(Some(sse)) = sse_stream.try_next().await {
        match sse {
            SSE::Event(event) => match serde_json::from_str::<EventData>(&event.data) {
                Ok(event_data) => simulation_sink.write_event(event_data),
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

        println!("Created and drained simulation {}", simulation_id);
    }

    Ok(())
}
