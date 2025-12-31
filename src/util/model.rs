use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormatType {
    Sql,
    Json,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Format {
    #[serde(rename = "type")]
    pub otype: FormatType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemImport {
    pub command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInferContext {
    pub description: Option<String>,
    pub command: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInfer {
    pub context: Option<SystemInferContext>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct System {
    pub key: String,
    pub format: Format,
    pub import: SystemImport,
    pub infer: Option<SystemInfer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySystem {
    #[serde(rename = "type")]
    pub stype: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub key: String,
    pub format: Option<Format>,
    pub system: Option<EntitySystem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Simulation {
    pub key: String,
    pub parent: String,
    pub seed: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SimulationRun {
    pub simulation: String,
    pub index: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SimulationRunData {
    pub simulation: String,
    pub index: u64,
    pub entities: Vec<Entity>,
    pub systems: Vec<System>,
}
