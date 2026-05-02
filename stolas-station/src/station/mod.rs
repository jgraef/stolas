pub mod antenna;
pub mod sensors;

use color_eyre::eyre::Error;
use serde::{
    Deserialize,
    Serialize,
};
use stolas_core::{
    AntennaConfig,
    ProcessingConfig,
    SdrConfig,
    SensorConfig,
};

use crate::station::{
    antenna::Antenna,
    sensors::Sensors,
};

#[derive(Clone, Debug)]
pub struct Station {
    antenna: Antenna,
    sensors: Sensors,
}

impl Station {
    pub async fn new(config: StationConfig) -> Result<Self, Error> {
        let antenna = Antenna::new(AntennaConfig {
            sdr: config.sdr,
            processing: config.processing,
        })
        .await?;
        let sensors = Sensors::new(config.sensors);

        Ok(Self { antenna, sensors })
    }

    pub fn antenna(&self) -> &Antenna {
        &self.antenna
    }

    pub fn sensors(&self) -> &Sensors {
        &self.sensors
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StationConfig {
    pub sdr: SdrConfig,

    pub processing: ProcessingConfig,

    pub sensors: SensorConfig,
}
