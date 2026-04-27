use std::time::Duration;

use stolas_core::{
    Frame,
    api::SensorValues,
};
use tokio::sync::{
    broadcast,
    watch,
};
use tokio_util::sync::CancellationToken;

use crate::sensors::spawn_sensor_task;

#[derive(Debug)]
pub struct Station {
    shutdown: CancellationToken,
    sensor_values: watch::Receiver<SensorValues>,
    //frames: broadcast::WeakSender<Frame>,
}

impl Station {
    pub fn new() -> Self {
        let shutdown = CancellationToken::new();

        // todo: put into config file
        let poll_interval = Duration::from_secs(1);

        let (_sensor_task, sensor_values) = spawn_sensor_task(poll_interval, shutdown.clone());

        Self {
            shutdown,
            sensor_values,
            //frames: todo!(),
        }
    }

    pub fn shutdown(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    pub fn sensor_values(&self) -> watch::Receiver<SensorValues> {
        self.sensor_values.clone()
    }

    pub fn frames(&self) -> broadcast::Receiver<Frame> {
        /*self.frames
        .upgrade()
        .expect("frames channel closed")
        .subscribe()*/
        todo!();
    }
}
