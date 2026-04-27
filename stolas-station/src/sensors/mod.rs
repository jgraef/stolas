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

use std::{
    ops::Deref,
    time::Duration,
};

use chrono::Utc;
use color_eyre::eyre::Error;
use stolas_core::api::SensorValues;
use systemstat::{
    DelayedMeasurement,
    Platform,
    data::CPULoad,
};
use tokio::{
    sync::watch,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{
    log_error_once,
    sensors::time::timedatectl_status,
};

pub mod time;

pub fn spawn_sensor_task(
    poll_interval: Duration,
    shutdown: CancellationToken,
) -> (JoinHandle<()>, watch::Receiver<SensorValues>) {
    let (sender, receiver) = watch::channel(SensorValues {
        time: Utc::now(),
        time_synced: false,
        cpu_temperature: None,
        cpu_load: None,
    });

    let task = tokio::spawn(async move {
        sensor_task(sender, poll_interval, shutdown).await.unwrap();
    });

    (task, receiver)
}

async fn sensor_task(
    sender: watch::Sender<SensorValues>,
    poll_interval: Duration,
    shutdown: CancellationToken,
) -> Result<(), Error> {
    let mut system = System::new();
    let mut poll_interval = tokio::time::interval(poll_interval);

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = poll_interval.tick() => {
                let mut sensor_values = sender.borrow().clone();
                poll_all(&mut sensor_values, &mut system).await;
                let _ = sender.send(sensor_values);
            },
        }
    }

    Ok(())
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
