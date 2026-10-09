//! Persistent CosKit project data. Snapshots never include another pipeline.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pipeline {
    pub schema: u32,
    pub head: Option<String>,
    pub versions: Vec<Version>,
    pub operations: Vec<Operation>,
    pub conversations: Vec<Value>,
    pub runs: Vec<Value>,
    #[serde(default)]
    pub omitted_operations: u64,
    #[serde(skip)]
    pub snapshots: BTreeMap<String, Arc<Vec<u8>>>,
    /// CKPipe v2 shared, compressed native objects, keyed by tiles/blobs path.
    #[serde(skip)]
    pub objects: BTreeMap<String, Arc<Vec<u8>>>,
    /// Small, independently decodable version previews (never authoritative pixels).
    #[serde(skip)]
    pub previews: BTreeMap<String, Arc<Vec<u8>>>,
}
impl Default for Pipeline {
    fn default() -> Self {
        Self {
            schema: 1,
            head: None,
            versions: vec![],
            operations: vec![],
            conversations: vec![],
            runs: vec![],
            omitted_operations: 0,
            snapshots: BTreeMap::new(),
            objects: BTreeMap::new(),
            previews: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub parent: Option<String>,
    pub label: String,
    pub snapshot: String,
    pub operation_count: usize,
    pub active_layer: Option<u64>,
    pub selected_layers: Vec<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub command: String,
    pub params: Value,
    pub base: Option<String>,
    pub active_layer: Option<u64>,
    pub selected_layers: Vec<u64>,
    pub replayable: bool,
}
