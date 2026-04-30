use crate::ai::run_prompt;
use crate::model::LocalSystem;
use crate::sim::load::load_systems_from_project_directory;
use anyhow::Result;
use reqwest::StatusCode;
use std::fs;
use std::path::Path;
use std::process::Command;

pub async fn infer(
    agent: Option<crate::config::AiAgent>,
    verbose: bool,
) -> Result<()> {
    let _ = dotenvy::dotenv();

    let config = crate::config::get_config()?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!(
            "{docs_url}/llm/skills/infer-effects.md",
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
        let system: LocalSystem = serde_json::from_value(system)?;
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
        "No systems in this project provide context, so you should infer effect definitions from migrations files, schema defintions and data access code."
    } else {
        "The remainder of this section contains context about each system for this application. You should use it to infer effect definitions."
    };

    let system_prompts = system_prompts.join("\n\n");
    let content = format!("{prompt}\n{inference_instructions}\n\n{system_prompts}");

    match agent {
        None => {
            println!("{}", content);
        }
        Some(agent) => {
            run_prompt(agent, &content, verbose, "effect inference")?;
            summarize_effects()?;
        }
    }

    Ok(())
}

fn summarize_effects() -> Result<()> {
    let effects_dir = Path::new(".rngo/effects");

    if !effects_dir.exists() {
        println!("No effects directory found at .rngo/effects");
        return Ok(());
    }

    let entries = fs::read_dir(effects_dir)?;
    let mut effect_files: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str());

            if (ext == Some("yml") || ext == Some("yaml"))
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                effect_files.push(stem.to_string());
            }
        }
    }

    effect_files.sort();

    if effect_files.is_empty() {
        println!("No effect definition files found in .rngo/effects/");
    } else {
        println!("Success! Effect definitions created:");
        for effect in effect_files {
            println!("  - {}", effect);
        }
        println!("Learn how to further customize effects at https://rngo.dev/docs/concepts/effect");
    }

    Ok(())
}
