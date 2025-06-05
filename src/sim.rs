use anyhow::{Result, anyhow, bail};
use eventsource_client::{Client, SSE};
use futures::TryStreamExt;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Simulation {
    id: String,
}

#[derive(Debug, Deserialize)]
struct EventData {
    stream: String,
    value: Value,
}

pub async fn sim(spec_path: String) -> Result<()> {
    let path = Path::new(&spec_path);

    if !path.exists() {
        bail!("Could not find file {}", spec_path)
    }

    let config = crate::util::get_config()?;
    let api_key = config
        .api_key
        .ok_or_else(|| anyhow!("Could not find API key"))?;

    let contents = fs::read_to_string(path)?;
    let json: Value = serde_yaml::from_str(&contents)?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{api_url}/simulations", api_url = config.api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json)
        .send()
        .await?;

    let simulation = response.json::<Simulation>().await?;

    let simulation_directory = format!(".rngo/simulations/{}", simulation.id);
    let simulation_directory = Path::new(&simulation_directory);
    fs::create_dir_all(simulation_directory)?;

    let client = eventsource_client::ClientBuilder::for_url(&format!(
        "{api_url}/simulations/{id}/stream",
        api_url = config.api_url,
        id = simulation.id
    ))?
    .header("Authorization", &format!("Bearer {}", api_key))?
    .build();

    let mut sse_stream = client.stream();
    let mut writers: HashMap<String, BufWriter<File>> = HashMap::new();

    while let Ok(Some(sse)) = sse_stream.try_next().await {
        match sse {
            SSE::Event(event) => match serde_json::from_str::<EventData>(&event.data) {
                Ok(event_data) => {
                    if !writers.contains_key(&event_data.stream) {
                        let stream_path =
                            simulation_directory.join(format!("{}.jsonl", event_data.stream));

                        let file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(stream_path.clone())
                            .expect(&format!("Failed to open file at {}", stream_path.display()));

                        writers.insert(event_data.stream.clone(), BufWriter::new(file));
                    }

                    let writer = writers.get_mut(&event_data.stream).expect("error");
                    writeln!(writer, "{}", event_data.value)?;
                }
                Err(_) => eprintln!("Failed to parse SSE data: {}", event.data),
            },
            SSE::Connected(_) => (),
            SSE::Comment(_) => (),
        }
    }

    println!(
        "Created simulation and drained to {}",
        simulation_directory.display()
    );

    Ok(())
}
