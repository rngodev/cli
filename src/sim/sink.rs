use crate::model::{EventData, FormatType, SimulationRunData};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct SimulationSink {
    effects: HashMap<String, Effect>,
    system_sinks: HashMap<String, Box<dyn Write>>,
    stream: bool,
    samples_sink: Option<Box<dyn Write>>,
}

#[derive(Debug)]
struct Effect {
    system_key: String,
    format_type: FormatType,
}

impl SimulationSink {
    pub fn stream() -> Self {
        SimulationSink {
            system_sinks: HashMap::new(),
            effects: HashMap::new(),
            stream: true,
            samples_sink: None,
        }
    }

    pub fn write_event(&mut self, event_data: EventData) {
        match &event_data {
            EventData::Effect { metadata, .. } if !metadata.is_empty() => {
                if let Some(ref mut sink) = self.samples_sink
                    && let Ok(json) = serde_json::to_string(&event_data)
                {
                    let _ = writeln!(sink, "{}", json);
                }
            }
            _ => {}
        }

        if let EventData::Error { .. } = event_data {
            if let Ok(str) = serde_json::to_string(&event_data) {
                eprintln!("Error: {}", str)
            }
        } else if self.stream {
            if let Ok(str) = serde_json::to_string(&event_data) {
                println!("{}", str)
            }
        } else if let EventData::Effect {
            effect,
            value,
            format,
            ..
        } = event_data
            && let Some(effect) = self.effects.get(&effect)
            && let Some(system_sink) = self.system_sinks.get_mut(&effect.system_key)
        {
            let value = match effect.format_type {
                FormatType::Json => &value.expect("value for JSON entities").to_string(),
                _ => &format.expect("format for non-JSON entities"),
            };

            let _ = writeln!(system_sink, "{}", value);
        }
    }
}

impl TryFrom<SimulationRunData> for SimulationSink {
    type Error = anyhow::Error;

    fn try_from(simulation_run_data: SimulationRunData) -> Result<Self> {
        // Load .env files before executing any commands
        let _ = dotenvy::dotenv();

        let simulation_directory = format!(".rngo/runs/{}", simulation_run_data.index);
        let simulation_directory = Path::new(&simulation_directory);

        let mut simulation_sink = SimulationSink {
            system_sinks: HashMap::new(),
            effects: HashMap::new(),
            stream: false,
            samples_sink: Some(Box::new(BufWriter::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(simulation_directory.join("samples.jsonl"))
                    .expect("Failed to open samples.jsonl"),
            ))),
        };

        // Track which systems have had their 'before' command run
        let mut systems_initialized: HashMap<String, ()> = HashMap::new();

        for effect in simulation_run_data.effects.iter() {
            if let Some(system_key) = &effect.system {
                let system = simulation_run_data
                    .systems
                    .iter()
                    .find(|s| s.key == *system_key)
                    .with_context(|| format!("Could not resolve system {}", system_key))?;

                #[cfg(target_os = "windows")]
                let (shell, flag) = ("cmd", "/C");

                #[cfg(not(target_os = "windows"))]
                let (shell, flag) = ("sh", "-c");

                // Run the 'before' command once per system if it exists
                if let Some(before_command) = &system.import.before
                    && !systems_initialized.contains_key(system_key.as_str())
                {
                    let status = Command::new(shell)
                        .arg(flag)
                        .arg(before_command)
                        .stdin(Stdio::null())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .status()
                        .with_context(|| {
                            format!(
                                "Could not run before command for system {}:\n\n{}",
                                system_key, before_command
                            )
                        })?;

                    if !status.success() {
                        anyhow::bail!(
                            "Before command failed for system {} with status: {}",
                            system_key,
                            status
                        );
                    }

                    systems_initialized.insert(system_key.clone(), ());
                }

                let mut child = Command::new(shell)
                    .arg(flag)
                    .arg(system.import.command.clone())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .with_context(|| {
                        format!(
                            "Could not run import command for system {}:\n\n{}",
                            system_key, system.import.command
                        )
                    })?;

                let child_stdin = child.stdin.take().expect("No stdin");

                let system_key = system_key.clone();

                simulation_sink.effects.insert(
                    effect.key.clone(),
                    Effect {
                        system_key: system_key.clone(),
                        format_type: system.format.otype.clone(),
                    },
                );

                simulation_sink
                    .system_sinks
                    .insert(system_key, Box::new(child_stdin));
            } else if let Some(format) = &effect.format {
                let (extension, system_type) = match format.otype {
                    FormatType::Sql => ("sql", "sql"),
                    FormatType::Json => ("jsonl", "json"),
                };

                let file_path = simulation_directory.join(format!("{}.{}", effect.key, extension));

                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path.clone())
                    .unwrap_or_else(|_| panic!("Failed to open file at {}", file_path.display()));

                let system_key = if let Some(entity) = &effect.entity {
                    format!("{}_{}", system_type, entity)
                } else {
                    format!("{}_{}", system_type, effect.key)
                };

                simulation_sink.effects.insert(
                    effect.key.clone(),
                    Effect {
                        system_key: system_key.clone(),
                        format_type: format.otype.clone(),
                    },
                );

                simulation_sink
                    .system_sinks
                    .insert(system_key, Box::new(BufWriter::new(file)));
            }
        }

        Ok(simulation_sink)
    }
}
