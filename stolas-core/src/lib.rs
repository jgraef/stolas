pub mod api;
pub mod file;
pub mod geo;

use std::{
    io::{
        Read,
        Write,
    },
    time::Duration,
};

use byteorder::{
    BigEndian,
    ReadBytesExt,
    WriteBytesExt,
};
use chrono::{
    DateTime,
    Utc,
};
use clap::Args;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Args)]
pub struct SdrConfig {
    #[clap(short = 's', long)]
    pub sdr_serial: Option<String>,

    #[clap(short = 'f', long, default_value = "1420405751")]
    pub center_frequency: u32,

    #[clap(short, long, default_value = "2400000")]
    pub sample_rate: u32,

    #[clap(short = 'g', long, default_value = "20.0")]
    pub tuner_gain: f32,

    #[clap(short = 't', long)]
    pub bias_tee: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Args)]
pub struct ProcessingConfig {
    #[clap(short, long, default_value = "512")]
    pub window_size: usize,

    #[clap(short, long, default_value = "50000")]
    pub average_size: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Args)]
pub struct AntennaConfig {
    #[clap(flatten)]
    #[serde(flatten)]
    pub sdr: SdrConfig,

    #[clap(flatten)]
    #[serde(flatten)]
    pub processing: ProcessingConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorConfig {
    pub poll_interval: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub serial: u64,
    pub timestamp: DateTime<Utc>,

    /// Power in linear, arbitrary units. The time interval and bandwidth over
    /// which this is depends on the configuration. The `Processing` struct in
    /// `stolas-station` has a comment about it, but we need to refine this to
    /// values that astronomers actually use and then document this properly.
    pub bins: Box<[f32]>,
}

impl Frame {
    pub fn stats(&self) -> FrameStats {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut average = 0.0;

        for x in self.bins.iter() {
            min = min.min(*x);
            max = max.max(*x);
            average += *x;
        }
        average /= self.bins.len() as f32;

        FrameStats { min, max, average }
    }

    pub fn read<R: Read>(mut reader: R) -> Result<Self, std::io::Error> {
        let serial = reader.read_u64::<BigEndian>()?;

        let timestamp_ns = reader.read_i64::<BigEndian>()?;
        let timestamp = DateTime::from_timestamp_nanos(timestamp_ns);

        let num_bins = reader.read_u32::<BigEndian>()?;
        let bins = (0..num_bins)
            .map(|_i| reader.read_f32::<BigEndian>())
            .collect::<Result<Box<[f32]>, _>>()?;

        Ok(Self {
            serial,
            timestamp,
            bins,
        })
    }

    pub fn write<W: Write>(&self, mut writer: W) -> Result<(), std::io::Error> {
        writer.write_u64::<BigEndian>(self.serial)?;

        let timestamp = self.timestamp.timestamp_nanos_opt().unwrap();
        writer.write_i64::<BigEndian>(timestamp)?;

        writer.write_u32::<BigEndian>(self.bins.len().try_into().unwrap())?;
        for value in self.bins.iter() {
            writer.write_f32::<BigEndian>(*value)?;
        }

        Ok(())
    }

    pub fn byte_length(&self) -> u32 {
        (self.bins.len() * 4).try_into().unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FrameStats {
    pub min: f32,
    pub max: f32,
    pub average: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AntennaEvent {
    ConfigChanged(AntennaConfig),
    Frame(Frame),
}
