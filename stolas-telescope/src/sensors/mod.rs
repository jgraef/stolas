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

use systemstat::{
    Platform,
    System,
};

pub mod time;

fn system() -> &'static System {
    // note: systemstat::platform::linux::PlatformImpl is a ZST, so we can always
    // get a 'static reference to it. But if this ever changes we can put it into a
    // lazy_static, or OnceLock.
    &systemstat::platform::linux::PlatformImpl
}

/// Current CPU temperature in °C
pub fn cpu_temperature() -> f32 {
    system().cpu_temp().unwrap()
}
