use std::collections::HashMap;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub station: StationConfig,

    #[serde(default)]
    pub antenna: AntennaConfig,

    // todo: move to database
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StationConfig {
    pub name: String,
    pub location: GeoLocation,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AntennaConfig {
    #[serde(default)]
    pub receiver: ReceiverConfig,
    pub default_profile: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ReceiverConfig {
    pub serial: Option<String>,

    #[serde(default)]
    pub bias_tee: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Profile {
    pub center_frequency: u32,
    pub sample_rate: u32,
    pub tuner_gain: f32,
    pub window_size: usize,
    pub average_size: usize,
}
