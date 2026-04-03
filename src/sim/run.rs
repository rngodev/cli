use crate::util::model::{Effect, SimulationRunData, System};
use anyhow::Result;
use reqwest::Client;

/// Downloads all simulation run data including effects and systems with pagination support.
///
/// This function handles pagination to ensure all effects and systems are downloaded,
/// regardless of the API's implicit limit.
pub async fn get_simulation_run_data(
    client: &Client,
    api_url: &str,
    api_key: &str,
    simulation_key: &str,
    run_index: u64,
) -> Result<SimulationRunData> {
    let effects = fetch_all_effects(client, api_url, api_key, simulation_key, run_index).await?;
    let systems = fetch_all_systems(client, api_url, api_key, simulation_key, run_index).await?;

    Ok(SimulationRunData {
        simulation: simulation_key.to_string(),
        index: run_index,
        effects,
        systems,
    })
}

/// Fetches all effects with pagination
async fn fetch_all_effects(
    client: &Client,
    api_url: &str,
    api_key: &str,
    simulation_key: &str,
    run_index: u64,
) -> Result<Vec<Effect>> {
    let mut all_effects = Vec::new();
    let mut offset = 0;
    let limit = 10;

    loop {
        let response = client
            .get(format!(
                "{api_url}/simulations/{simulation_key}/runs/{run_index}/effects"
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .query(&[("offset", offset.to_string()), ("limit", limit.to_string())])
            .send()
            .await?;

        let effects = response.json::<Vec<Effect>>().await?;
        let count = effects.len();

        all_effects.extend(effects);

        // If we received fewer items than the limit, we've reached the end
        if count < limit {
            break;
        }

        offset += limit;
    }

    Ok(all_effects)
}

/// Fetches all systems with pagination
async fn fetch_all_systems(
    client: &Client,
    api_url: &str,
    api_key: &str,
    simulation_key: &str,
    run_index: u64,
) -> Result<Vec<System>> {
    let mut all_systems = Vec::new();
    let mut offset = 0;
    let limit = 10;

    loop {
        let response = client
            .get(format!(
                "{api_url}/simulations/{simulation_key}/runs/{run_index}/systems"
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .query(&[("offset", offset.to_string()), ("limit", limit.to_string())])
            .send()
            .await?;

        let systems = response.json::<Vec<System>>().await?;
        let count = systems.len();

        all_systems.extend(systems);

        // If we received fewer items than the limit, we've reached the end
        if count < limit {
            break;
        }

        offset += limit;
    }

    Ok(all_systems)
}
