pub mod antenna;
pub mod captures;
pub mod sensors;

use std::path::Path;

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

use crate::{
    database::Database,
    station::{
        antenna::Antenna,
        captures::Captures,
        sensors::Sensors,
    },
};

#[derive(Clone, Debug)]
pub struct Station {
    antenna: Antenna,
    sensors: Sensors,
    captures: Captures,
}

impl Station {
    pub async fn new(
        config: StationConfig,
        database: Database,
        data_path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let antenna = Antenna::new(AntennaConfig {
            sdr: config.sdr,
            processing: config.processing,
        })
        .await?;

        let sensors = Sensors::new(config.sensors);

        let captures = Captures::new(database, data_path.as_ref().join("captures"))?;

        Ok(Self {
            antenna,
            sensors,
            captures,
        })
    }

    pub fn antenna(&self) -> &Antenna {
        &self.antenna
    }

    pub fn sensors(&self) -> &Sensors {
        &self.sensors
    }

    pub fn captures(&self) -> &Captures {
        &self.captures
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StationConfig {
    pub sdr: SdrConfig,

    pub processing: ProcessingConfig,

    pub sensors: SensorConfig,
}
