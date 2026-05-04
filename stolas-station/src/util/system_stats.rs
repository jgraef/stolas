// todo: rewrite as background task and provide a tokio::watch::Receiver with
// cpu temperature, laod, etc.

use std::ops::Deref;

use color_eyre::eyre::Error;
use systemstat::{
    CPULoad,
    DelayedMeasurement,
    Platform,
};

/// [`systemstat::System`] wrapper that buffers
/// [`DelayedMeasurement`s][DelayedMeasurement], so that you don't have to wait
/// to get a result.
#[derive(derive_more::Debug)]
pub struct System {
    #[debug(skip)]
    system: systemstat::System,
    #[debug(skip)]
    cpu_load: Option<DelayedMeasurement<Vec<CPULoad>>>,
}

impl System {
    pub fn new() -> Self {
        Self {
            system: systemstat::System,
            cpu_load: None,
        }
    }

    pub fn cpu_load(&mut self) -> Result<Option<Vec<CPULoad>>, Error> {
        let value = self
            .cpu_load
            .take()
            .map(|delayed| delayed.done())
            .transpose()?;

        self.cpu_load = Some(self.system.cpu_load()?);

        Ok(value)
    }
}

impl Deref for System {
    type Target = systemstat::System;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}
