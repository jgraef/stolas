use chrono::{
    DateTime,
    Utc,
};
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum ApiError {
    // todo
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
