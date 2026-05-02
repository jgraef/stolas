//! Auxilary sensors
//!
//! This module contains code for auxilary sensors, such as time-keeping,
//! location, pointing, etc.
//!
//! # TODO
//!
//! - Location/Time via GPS
//! - Pointing via Accelerometer/Compass module
//! - Time via RTC module

pub mod time;

use std::{
    ops::Deref,
    sync::Arc,
};

use chrono::Utc;
use color_eyre::eyre::Error;
use futures_util::FutureExt;
use stolas_core::{
    SensorConfig,
    api::SensorValues,
};
use systemstat::{
    DelayedMeasurement,
    Platform,
    data::CPULoad,
};
use tokio::{
    sync::watch,
    task::JoinHandle,
};
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::{
    log_error_once,
    station::sensors::time::timedatectl_status,
};

#[derive(Clone, Debug)]
pub struct Sensors {
    receiver: watch::Receiver<SensorValues>,
    #[allow(unused)]
    shared: Arc<Shared>,
}

#[derive(Debug)]
#[allow(unused)]
struct Shared {
    drop_guard: DropGuard,
    join_handle: JoinHandle<()>,
}

impl Sensors {
    pub fn new(config: SensorConfig) -> Self {
        let (sender, receiver) = watch::channel(SensorValues {
            time: Utc::now(),
            time_synced: false,
            cpu_temperature: None,
            cpu_load: None,
        });

        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();
        let mut system = System::new();
        let mut poll_interval = tokio::time::interval(config.poll_interval);

        let join_handle = tokio::spawn(
            shutdown
                .run_until_cancelled_owned(async move {
                    tracing::debug!("starting sensor task");

                    loop {
                        poll_interval.tick().await;

                        let mut sensor_values = sender.borrow().clone();
                        poll_all(&mut sensor_values, &mut system).await;
                        let _ = sender.send(sensor_values);
                    }
                })
                .map(|_| {
                    tracing::debug!("sensor task stopped");
                }),
        );

        Self {
            receiver,
            shared: Arc::new(Shared {
                drop_guard,
                join_handle,
            }),
        }
    }

    pub fn sensor_values(&self) -> watch::Receiver<SensorValues> {
        self.receiver.clone()
    }
}

async fn poll_all(sensor_values: &mut SensorValues, system: &mut System) {
    sensor_values.time = Utc::now();
    if !sensor_values.time_synced {
        if timedatectl_status()
            .await
            .is_ok_and(|status| status.ntp_synchronized)
        {
            tracing::info!("Time synchronized");
            sensor_values.time_synced = true;
        }
    }

    sensor_values.cpu_temperature = log_error_once!(system.cpu_temp()).ok();
    sensor_values.cpu_load = system.cpu_load().ok().flatten().map(|cpu_loads| {
        cpu_loads
            .into_iter()
            .map(|cpu_load| cpu_load.user)
            .collect()
    });
}

/// [`systemstat::System`] wrapper that buffers
/// [`DelayedMeasurement`s][DelayedMeasurement], so that you don't have to wait
/// to get a result.
#[derive(derive_more::Debug)]
struct System {
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
