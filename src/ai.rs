use anyhow::Result;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::config::{AiAgent, Config};

pub fn run_prompt(
    config: &Config,
    content: &str,
    verbose: bool,
    agent_context: &str,
    agent_override: Option<AiAgent>,
) -> Result<()> {
    let agent = agent_override
        .or_else(|| config.ai.as_ref().map(|ai| ai.agent.clone()))
        .ok_or_else(|| anyhow::anyhow!(
            "AI agent must be configured to run this command.\n\
             Use the --agent flag or set ai.agent in your .rngo/config.yml or user config file:\n\n\
             ai:\n  agent: claude  # or codex, or copilot"
        ))?;

    let (cli_name, agent_name) = match agent {
        AiAgent::Claude => ("claude", "Claude Code"),
        AiAgent::Codex => ("codex", "Codex"),
        AiAgent::Copilot => ("copilot", "Copilot"),
    };

    println!("Running {} prompt in {}...\n", agent_context, agent_name);

    let mut cmd = Command::new(cli_name);
    let needs_stdin = match agent {
        AiAgent::Claude => {
            cmd.arg("-p").arg("--permission-mode").arg("acceptEdits");
            true
        }
        AiAgent::Codex => {
            cmd.arg("exec").arg("--full-auto");
            true
        }
        AiAgent::Copilot => {
            cmd.arg("-p").arg(content).arg("--allow-all-tools");
            false
        }
    };

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

    if needs_stdin && let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("{} exited with status: {}", agent_name, status)
    }

    Ok(())
}
