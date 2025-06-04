use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Simulation {
    id: String,
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
        .post("http://localhost:8001/simulations")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json)
        .send()
        .await?;

    let simulation = response.json::<Simulation>().await?;

    println!("Created simulation ID {}", simulation.id);

    Ok(())
}
