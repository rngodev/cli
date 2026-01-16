use crate::util::ai::run_prompt;
use anyhow::Result;
use reqwest::StatusCode;
use std::fs;
use std::path::Path;

pub async fn infer_systems(prompt_only: bool, verbose: bool) -> Result<()> {
    let config = crate::util::config::get_config()?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!(
            "{docs_url}/llm/skills/infer-systems.md",
            docs_url = config.docs_url
        ))
        .send()
        .await?;

    if response.status() != StatusCode::OK {
        anyhow::bail!("Failed to download system inference prompt")
    }

    let content = response.text().await?;

    // If --prompt flag is set, just output the prompt and exit
    if prompt_only {
        println!("{}", content);
        return Ok(());
    }

    // Run the prompt through the configured AI agent
    run_prompt(&config, &content, verbose, "system inference")?;

    // Summarize results
    summarize_systems()?;

    Ok(())
}

fn summarize_systems() -> Result<()> {
    let systems_dir = Path::new(".rngo/systems");

    if !systems_dir.exists() {
        println!("No systems directory found at .rngo/systems");
        return Ok(());
    }

    let entries = fs::read_dir(systems_dir)?;
    let mut system_files: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str());

            if (ext == Some("yml") || ext == Some("yaml"))
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                system_files.push(stem.to_string());
            }
        }
    }

    system_files.sort();

    if system_files.is_empty() {
        println!("No system definition files found in .rngo/systems/");
    } else {
        println!("Success! System definitions created:");
        for system in system_files {
            println!("  - {}", system);
        }
        println!("Learn how to further customize systems at https://rngo.dev/docs/concepts/system");
    }

    Ok(())
}
