use std::time::Duration;

use chrono::{
    DateTime,
    FixedOffset,
};
use color_eyre::eyre::{
    Error,
    bail,
    eyre,
};
use tokio::process::Command;

pub async fn wait_for_time_sync() -> Result<(), Error> {
    tracing::info!("Waiting for time-synchronization");

    loop {
        let status = timedatectl_status().await?;
        tracing::debug!(?status);

        if status.ntp_synchronized {
            // clock is synchronized
            tracing::info!("Time synchronized");

            return Ok(());
        }

        if !status.ntp {
            // ntp is disabled and thus will never be synchronized
            bail!("NTP is not enabled");
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

pub async fn timedatectl_status() -> Result<TimeDateCtlStatus, Error> {
    let output = Command::new("timedatectl")
        .args(["--no-ask-password", "show"])
        .output()
        .await?;

    if !output.status.success() {
        bail!("timedatectl failed with status code {:?}", output.status);
    }
    let output = str::from_utf8(&output.stdout)?.trim();

    let mut status = TimeDateCtlStatus::default();

    fn parse_bool(s: &str) -> Result<bool, Error> {
        Ok(s == "yes")
    }

    // fixme: this somehow doesn't work
    /*fn parse_time(s: &str) -> Result<DateTime<FixedOffset>, Error> {
        tracing::debug!(?s);
        Ok(DateTime::parse_from_str(s, "%a %Y-%m-%d %H:%M:%S %Z")?)
    }*/

    for line in output.lines() {
        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| eyre!("Unexpected output from timedatectl"))?;

        match k {
            "LocalRTC" => status.local_rtc = parse_bool(v)?,
            "CanNTP" => status.can_ntp = parse_bool(v)?,
            "NTP" => status.ntp = parse_bool(v)?,
            "NTPSynchronized" => status.ntp_synchronized = parse_bool(v)?,
            //"TimeUSec" => status.time_usec = Some(parse_time(v)?),
            //"RTCTimeUSec" => status.rtc_time_usec = Some(parse_time(v)?),
            _ => {}
        }
    }

    Ok(status)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TimeDateCtlStatus {
    pub local_rtc: bool,
    pub can_ntp: bool,
    pub ntp: bool,
    pub ntp_synchronized: bool,
    pub time_usec: Option<DateTime<FixedOffset>>,
    pub rtc_time_usec: Option<DateTime<FixedOffset>>,
}
