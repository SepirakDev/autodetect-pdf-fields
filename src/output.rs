use serde::Serialize;

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
    pub page: usize,
    pub confidence: f32,
    #[serde(flatten)]
    pub bbox: BBox,
}
