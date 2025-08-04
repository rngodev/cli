use crate::util::model::System;
use crate::util::spec::load_systems_from_project_directory;
use anyhow::Result;
use reqwest::StatusCode;
use std::process::Command;

pub async fn infer() -> Result<()> {
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

            match context_parts {
                (Some(description), _) => {
                    system_prompt.push_str(&format!("\n\n{description}"));
                }
                _ => (),
            }

            match context_parts {
                (_, Some(command)) => {
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
                _ => (),
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
    println!("{prompt}\n{inference_instructions}\n\n{system_prompts}");

    Ok(())
}
