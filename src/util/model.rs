use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormatType {
    Sql,
    Json,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Format {
    #[serde(rename = "type")]
    pub otype: FormatType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SystemImport {
    pub command: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SystemInferContext {
    pub description: Option<String>,
    pub command: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SystemInfer {
    pub context: Option<SystemInferContext>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct System {
    pub format: Format,
    pub import: SystemImport,
    pub infer: Option<SystemInfer>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntitySystem {
    #[serde(rename = "type")]
    pub stype: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Entity {
    pub format: Option<Format>,
    pub system: Option<EntitySystem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Spec {
    pub entities: HashMap<String, Entity>,
    #[serde(default)]
    pub systems: HashMap<String, System>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Simulation {
    pub key: String,
    pub id: String,
    #[serde(flatten)]
    pub spec: Spec,
}
