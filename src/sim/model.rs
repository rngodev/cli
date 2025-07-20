use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputType {
    Sql,
    Json,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Output {
    #[serde(rename = "type")]
    pub otype: OutputType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SystemImport {
    pub command: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct System {
    pub output: Output,
    pub import: SystemImport,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntitySystem {
    #[serde(rename = "type")]
    pub stype: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Entity {
    pub output: Option<Output>,
    pub system: Option<EntitySystem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Simulation {
    pub id: String,
    pub entities: HashMap<String, Entity>,
    pub systems: HashMap<String, System>,
}
