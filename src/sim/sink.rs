use crate::sim::{EventData, Simulation};
use crate::util::model::OutputType;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct SimulationSink {
    entities: HashMap<String, Entity>,
    system_sinks: HashMap<String, Box<dyn Write>>,
    stream: bool,
}

struct Entity {
    system_key: String,
    output_type: OutputType,
}

impl SimulationSink {
    pub fn stream() -> Self {
        SimulationSink {
            system_sinks: HashMap::new(),
            entities: HashMap::new(),
            stream: true,
        }
    }

    pub fn write_event(&mut self, event_data: EventData) {
        if let EventData::Error { .. } = event_data {
            match serde_json::to_string(&event_data) {
                Ok(str) => eprintln!("Error: {}", str),
                Err(_) => (),
            }
        } else if self.stream {
            match serde_json::to_string(&event_data) {
                Ok(str) => println!("{}", str),
                Err(_) => (),
            }
        } else if let EventData::Create { entity, value, .. } = event_data {
            if let Some(entity) = self.entities.get(&entity) {
                if let Some(system_sink) = self.system_sinks.get_mut(&entity.system_key) {
                    let value = match entity.output_type {
                        OutputType::Json => &value.to_string(),
                        _ => value.as_str().unwrap(),
                    };

                    let _ = writeln!(system_sink, "{}", value);
                }
            }
        }
    }
}

impl TryFrom<Simulation> for SimulationSink {
    type Error = anyhow::Error;

    fn try_from(simulation: Simulation) -> Result<Self> {
        let mut simulation_sink = SimulationSink {
            system_sinks: HashMap::new(),
            entities: HashMap::new(),
            stream: false,
        };

        let simulation_directory = format!(".rngo/simulations/{}", simulation.id);
        let simulation_directory = Path::new(&simulation_directory);

        for (key, entity) in simulation.spec.entities.iter() {
            if let Some(entity_system) = &entity.system {
                let system = simulation
                    .spec
                    .systems
                    .get(&entity_system.stype)
                    .with_context(|| {
                        format!("Could not resolve system type {}", entity_system.stype)
                    })?;

                let command_parts: Vec<&str> = system.import.command.split_whitespace().collect();

                let mut child = Command::new(command_parts[0].to_string())
                    .args(&command_parts[1..])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::inherit())
                    .spawn()
                    .with_context(|| {
                        format!(
                            "Could not run import command for system {}:\n\n{}",
                            entity_system.stype, system.import.command
                        )
                    })?;

                let child_stdin = child.stdin.take().expect("No stdin");

                let system_key = entity_system.stype.clone();

                simulation_sink.entities.insert(
                    key.clone(),
                    Entity {
                        system_key: system_key.clone(),
                        output_type: system.output.otype.clone(),
                    },
                );

                simulation_sink
                    .system_sinks
                    .insert(system_key, Box::new(child_stdin));
            } else if let Some(output) = &entity.output {
                let (extension, system_type) = match output.otype {
                    OutputType::Sql => ("sql", "sql"),
                    OutputType::Json => ("jsonl", "json"),
                };

                let file_path = simulation_directory.join(format!("{}.{}", key, extension));

                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path.clone())
                    .expect(&format!("Failed to open file at {}", file_path.display()));

                let system_key = format!("{}_{}", system_type, key);

                simulation_sink.entities.insert(
                    key.clone(),
                    Entity {
                        system_key: system_key.clone(),
                        output_type: output.otype.clone(),
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
