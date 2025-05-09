use serde::{Deserialize, Serialize};
type TableColumn = Vec<String>;

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Table {
    #[serde(skip_serializing, skip_deserializing)]
    pub identifier: String,
    pub rows: Vec<TableColumn>,
    pub scroll_pos: f32,
}
