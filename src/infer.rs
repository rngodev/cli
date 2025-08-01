use crate::InferCommands;
use anyhow::Result;
use reqwest::StatusCode;

pub async fn infer(command: InferCommands) -> Result<()> {
    let config = crate::util::get_config()?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!(
            "{docs_url}/llm/infer.md",
            docs_url = config.docs_url
        ))
        .send()
        .await?;

    if response.status() != StatusCode::OK {
        anyhow::bail!("Failed to download latest prompt")
    }

    let prompt = response.text().await?;

    println!("{prompt}");

    Ok(())
}
