use serde::{Deserialize, Serialize};
pub type TableRow = Vec<String>;

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Table {
    #[serde(skip_serializing, skip_deserializing)]
    pub identifier: String,
    pub rows: Vec<TableRow>,
    pub scroll_pos: f32,
}
