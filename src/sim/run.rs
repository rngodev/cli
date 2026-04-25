use crate::model::{EventData, Simulation, SimulationRun};
use crate::sim::problem::Problem;
use crate::sim::sink::SimulationSink;
use crate::sim::{api, load};
use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub async fn run(file: Option<String>, stdout: bool) -> Result<()> {
    let config = crate::config::get_config()?;
    let api_key = config
        .api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Could not find API key"))?;

    let client = reqwest::Client::new();

    let (key, sim) = {
        let mut sim = if let Some(file) = file {
            load::load_sim_from_file(file)?
        } else {
            load::load_sim_from_project_directory(&config)?
        };

        if let Value::Object(ref mut map) = sim {
            map.insert("output".into(), "stream".into());
            let key = map
                .remove("key")
                .ok_or_else(|| anyhow!("simulation must have a key"))?
                .as_str()
                .ok_or_else(|| anyhow!("simulation key must be a string"))?
                .to_string();

            (key, sim)
        } else {
            bail!("simulation is not an object")
        }
    };

    let simulation = {
        let push_simulation_response = client
            .put(format!(
                "{api_url}/simulations/{key}",
                api_url = config.api_url,
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&sim)
            .send()
            .await?;

        if !push_simulation_response.status().is_success() {
            let status = push_simulation_response.status();
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
                "{api_url}/simulations/{simulation_key}/runs",
                api_url = config.api_url,
                simulation_key = simulation.key
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

    let simulation_run_directory = format!(".rngo/runs/{}", simulation_run.index);
    let simulation_run_directory = Path::new(&simulation_run_directory);

    if !stdout {
        fs::create_dir_all(simulation_run_directory)?;

        let last_symlink = Path::new(".rngo/runs/last");
        if last_symlink.symlink_metadata().is_ok() {
            fs::remove_file(last_symlink)?;
        }
        let symlink_result = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(simulation_run.index.to_string(), last_symlink)
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(simulation_run.index.to_string(), last_symlink)
            }
        };
        if let Err(e) = symlink_result {
            eprintln!(
                "Warning: could not create symlink at {}: {}",
                last_symlink.display(),
                e
            );
        }
    }

    let simulation_run_data = api::get_simulation_run_data(
        &client,
        &config.api_url,
        api_key,
        &simulation_run.simulation,
        simulation_run.index,
    )
    .await?;

    let mut simulation_sink = if stdout {
        SimulationSink::stream()
    } else {
        SimulationSink::try_from(simulation_run_data.clone())?
    };

    let stream_url = format!(
        "{api_url}/simulations/{simulation_key}/runs/{run_index}/stream",
        api_url = config.api_url,
        simulation_key = simulation_run.simulation,
        run_index = simulation_run.index
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
                                EventData::Effect { id, .. } => *id,
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
        let effects_map: serde_json::Map<String, Value> = simulation_run_data
            .effects
            .into_iter()
            .map(|effect| {
                let key = effect.key.clone();
                let mut value = serde_json::to_value(effect).unwrap();
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
        spec.insert("effects".to_string(), json!(effects_map));
        spec.insert("systems".to_string(), json!(systems_map));

        let spec_path = simulation_run_directory.join("spec.yml");
        fs::write(spec_path, serde_json::to_string_pretty(&spec)?)?;

        println!("Created and ran simulation");
        println!("  fs:  .rngo/runs/{}", simulation_run.index);
        println!("  sim: https://rngo.dev/simulations/{}", simulation.key);
        println!(
            "  run: https://rngo.dev/simulations/{}/runs/{}",
            simulation.key, simulation_run.index
        );
    }

    Ok(())
}
