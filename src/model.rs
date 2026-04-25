use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub before: Option<String>,
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
pub struct LocalSystem {
    pub format: Format,
    pub import: SystemImport,
    pub infer: Option<SystemInfer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub key: String,
    pub system: Option<String>,
    pub entity: Option<String>,
    pub format: Option<Format>,
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
    pub index: u64,
    pub effects: Vec<Effect>,
    pub systems: Vec<System>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum EventData {
    Effect {
        id: u64,
        effect: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        offset: i64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        metadata: Vec<Metadata>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
    Error {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        effect: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<Vec<String>>,
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    tag: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    path: Vec<String>,
}
