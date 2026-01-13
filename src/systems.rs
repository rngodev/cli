use anyhow::Result;
use reqwest::StatusCode;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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

    println!("Running system inference prompt in Claude Code...\n");

    // Spawn claude process with stdin piped
    // In verbose mode, inherit stdout to see Claude's output
    let mut child = Command::new("claude")
        .arg("-p")
        .arg("--permission-mode")
        .arg("acceptEdits")
        .stdin(Stdio::piped())
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::inherit())
        .spawn()?;

    // Write the content to claude's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    // Wait for the process to complete
    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("claude exited with status: {}", status)
    }

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

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str());

                if ext == Some("yml") || ext == Some("yaml") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        system_files.push(stem.to_string());
                    }
                }
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
    }

    Ok(())
}
