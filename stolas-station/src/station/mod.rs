pub mod antenna;
pub mod captures;
pub mod sensors;

use std::path::Path;

use color_eyre::eyre::{
    Error,
    eyre,
};

use crate::{
    config::Config,
    database::Database,
    station::{
        antenna::{
            Antenna,
            PipelineOptions,
            ReceiverOptions,
        },
        captures::Captures,
    },
};

#[derive(Clone, Debug)]
pub struct Station {
    antenna: Antenna,
    captures: Captures,
}

impl Station {
    pub async fn new(
        config: Config,
        database: Database,
        data_path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let antenna = Antenna::new(config.antenna.clone())?;

        if let Some(default_profile) = &config.antenna.default_profile {
            let profile = config
                .profiles
                .get(default_profile)
                .ok_or_else(|| eyre!("Profile not found: {default_profile}"))?;

            antenna
                .start(
                    ReceiverOptions {
                        center_frequency: profile.center_frequency,
                        sample_rate: profile.sample_rate,
                        tuner_gain: profile.tuner_gain,
                    },
                    PipelineOptions {
                        window_size: profile.window_size,
                        average_size: profile.average_size,
                    },
                )
                .await?;
        }

        let captures = Captures::new(database, data_path.as_ref().join("captures"))?;

        Ok(Self { antenna, captures })
    }

    pub fn antenna(&self) -> &Antenna {
        &self.antenna
    }

    pub fn captures(&self) -> &Captures {
        &self.captures
    }
}
