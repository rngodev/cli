use anyhow::Result;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::util::config::Config;

/// Runs a prompt through the configured AI agent CLI
///
/// # Arguments
/// * `config` - The application config containing AI agent settings
/// * `content` - The prompt content to send to the agent
/// * `verbose` - Whether to show the agent's output
/// * `agent_context` - Context string for error messages (e.g., "entity inference", "system inference")
pub fn run_prompt(
    config: &Config,
    content: &str,
    verbose: bool,
    agent_context: &str,
) -> Result<()> {
    // Check if ai.agent is configured and determine which CLI to use
    let (cli_name, agent_name) = match &config.ai {
        Some(ai_config) => match ai_config.agent {
            crate::util::config::AiAgent::Claude => ("claude", "Claude Code"),
            crate::util::config::AiAgent::Codex => ("codex", "Codex"),
            crate::util::config::AiAgent::Copilot => ("copilot", "Copilot"),
        },
        None => {
            anyhow::bail!(
                "AI agent must be configured to run this command.\n\
                 Please set ai.agent in your .rngo/config.yml or user config file:\n\n\
                 ai:\n  agent: claude  # or codex, or copilot"
            );
        }
    };

    println!("Running {} prompt in {}...\n", agent_context, agent_name);

    // Build the command with appropriate arguments for each agent
    let mut cmd = Command::new(cli_name);
    let needs_stdin = match &config.ai {
        Some(ai_config) => match ai_config.agent {
            crate::util::config::AiAgent::Claude => {
                cmd.arg("-p").arg("--permission-mode").arg("acceptEdits");
                true // Claude receives prompt via stdin
            }
            crate::util::config::AiAgent::Codex => {
                cmd.arg("exec").arg("--full-auto");
                true // Codex receives prompt via stdin
            }
            crate::util::config::AiAgent::Copilot => {
                cmd.arg("-p").arg(content).arg("--allow-all-tools");
                false // Copilot receives prompt as argument
            }
        },
        None => unreachable!(), // Already checked above
    };

    // Spawn the CLI process
    // In verbose mode, inherit stdout to see the agent's output
    let mut child = cmd
        .stdin(if needs_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .spawn()?;

    // Write the content to the agent's stdin if needed
    if needs_stdin && let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    // Wait for the process to complete
    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("{} exited with status: {}", agent_name, status)
    }

    Ok(())
}
