use chrono::{
    DateTime,
    Utc,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    AntennaConfig,
    Frame,
};

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
#[error("Internal server error: {message}")]
pub struct InternalError {
    pub message: String,
    pub backtrace: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorValues {
    pub time: DateTime<Utc>,
    pub time_synced: bool,
    pub cpu_temperature: Option<f32>,
    pub cpu_load: Option<Vec<f32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StatusEvent {
    Sensors(SensorValues),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AntennaMessage {
    Config(AntennaConfig),
    Frame(Frame),
    Lagged { lag: u64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureEntry {
    pub file_name: String,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub active: bool,
    // todo: other meta data
}
