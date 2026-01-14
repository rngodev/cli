use crate::util::ai::run_prompt;
use crate::util::model::System;
use crate::util::spec::load_systems_from_project_directory;
use anyhow::Result;
use reqwest::StatusCode;
use std::fs;
use std::path::Path;
use std::process::Command;

pub async fn infer_entities(prompt_only: bool, verbose: bool) -> Result<()> {
    let config = crate::util::config::get_config()?;
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

    let systems = load_systems_from_project_directory()?;
    let mut system_prompts = vec![];

    for (key, system) in systems {
        let system: System = serde_json::from_value(system)?;
        let context_parts = system
            .infer
            .and_then(|infer| infer.context)
            .map(|context| (context.description, context.command));

        if let Some(context_parts) = context_parts {
            let mut system_prompt = format!("### System {key}");

            if let (Some(description), _) = context_parts {
                system_prompt.push_str(&format!("\n\n{description}"));
            }

            if let (_, Some(command)) = context_parts {
                #[cfg(target_os = "windows")]
                let (shell, flag) = ("cmd", "/C");

                #[cfg(not(target_os = "windows"))]
                let (shell, flag) = ("sh", "-c");

                let output = Command::new(shell).arg(flag).arg(command).output()?;
                if output.status.success() {
                    let output = String::from_utf8_lossy(&output.stdout);
                    system_prompt.push_str(&format!("\n\n```\n{output}\n```"))
                }
            }

            system_prompts.push(system_prompt)
        }
    }

    let inference_instructions = if system_prompts.is_empty() {
        "No systems in this project provide context, so you should infer entity definitions from migrations files, schema defintions and data access code."
    } else {
        "The remainder of this section contains context about each system for this application. You should use it to infer entity definitions."
    };

    let system_prompts = system_prompts.join("\n\n");
    let content = format!("{prompt}\n{inference_instructions}\n\n{system_prompts}");

    // If --prompt flag is set, just output the prompt and exit
    if prompt_only {
        println!("{}", content);
        return Ok(());
    }

    // Run the prompt through the configured AI agent
    run_prompt(&config, &content, verbose, "entity inference")?;

    // Summarize results
    summarize_entities()?;

    Ok(())
}

fn summarize_entities() -> Result<()> {
    let entities_dir = Path::new(".rngo/entities");

    if !entities_dir.exists() {
        println!("No entities directory found at .rngo/entities");
        return Ok(());
    }

    let entries = fs::read_dir(entities_dir)?;
    let mut entity_files: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str());

            if (ext == Some("yml") || ext == Some("yaml"))
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                entity_files.push(stem.to_string());
            }
        }
    }

    entity_files.sort();

    if entity_files.is_empty() {
        println!("No entity definition files found in .rngo/entities/");
    } else {
        println!("Success! Entity definitions created:");
        for entity in entity_files {
            println!("  - {}", entity);
        }
        println!(
            "Learn how to further customize entities at https://rngo.dev/docs/concepts/entity"
        );
    }

    Ok(())
}
