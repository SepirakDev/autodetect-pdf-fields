use serde::{Deserialize, Serialize};

use crate::geometry::BBox;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Checkbox,
    Date,
    Signature,
    Number,
}

impl FieldType {
    pub fn from_class_id(id: i32) -> Self {
        match id {
            0 => FieldType::Text,
            1 => FieldType::Checkbox,
            _ => FieldType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedField {
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_id: Option<String>,
    pub page: usize,
    pub confidence: f32,
    #[serde(flatten)]
    pub bbox: BBox,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AvailableField {
    #[serde(rename = "type")]
    pub field_type: String,
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AvailableFieldsFile {
    #[serde(rename = "availableFields")]
    pub available_fields: Vec<AvailableField>,
}
