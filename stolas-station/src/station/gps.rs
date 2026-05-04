use chrono::{
    DateTime,
    Utc,
};
use color_eyre::eyre::{
    Error,
    bail,
};
use serde::{
    Deserialize,
    Serialize,
};
use tokio::{
    io::{
        AsyncBufReadExt,
        AsyncWriteExt,
        BufReader,
        BufWriter,
        Lines,
    },
    net::{
        TcpStream,
        tcp::{
            OwnedReadHalf,
            OwnedWriteHalf,
        },
    },
    sync::{
        mpsc,
        watch,
    },
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpsConfig {
    #[serde(default = "default_address")]
    pub address: String,
}

fn default_address() -> String {
    "localhost:2947".to_owned()
}

impl Default for GpsConfig {
    fn default() -> Self {
        Self {
            address: default_address(),
        }
    }
}

type FixChannelItem = Result<Option<gpsd_proto::Tpv>, gpsd_proto::Error>;

#[derive(Clone, Debug)]
pub struct Fix {
    pub time: DateTime<Utc>,

    /// Latitude in degrees: +/- signifies North/South.
    pub latitude: f64,

    /// Longitude in degrees: +/- signifies East/West.
    pub longitude: f64,

    /// Altitude, height above ellipsoid, in meters. Probably WGS84.
    pub alt: f64,
}

#[derive(Clone, Debug)]
pub struct Gps {
    #[allow(unused)]
    command_sender: mpsc::Sender<Command>,
    fix_receiver: watch::Receiver<FixChannelItem>,
}

impl Gps {
    pub async fn new(gps_config: &GpsConfig) -> Result<Self, Error> {
        tracing::info!(address = gps_config.address, "Connecting to gpsd");

        let stream = TcpStream::connect(&gps_config.address).await?;

        let (command_sender, command_receiver) = mpsc::channel(16);
        let (fix_sender, fix_receiver) = watch::channel(Ok(None));

        let task = GpsdReceiverTask::new(stream, command_receiver, fix_sender).await?;

        let _join_handle = tokio::spawn(async move {
            if let Err(error) = task.run().await {
                tracing::error!(%error);
                todo!("handle error by returning it to caller of getter");
            }
        });

        Ok(Self {
            command_sender,
            fix_receiver,
        })
    }

    pub async fn get(&mut self) -> Result<Fix, Error> {
        loop {
            // wait for a new value
            if self.fix_receiver.changed().await.is_err() {
                // closed and seen
                //
                // todo: should we just await forever here?
                bail!("gpsd shutdown");
            }

            let fix = &*self.fix_receiver.borrow_and_update();
            match fix {
                Err(error) => {
                    // error from gpsd
                    bail!("gpsd error: {}", error.message);
                }
                Ok(None) => {
                    // no message yet
                }
                Ok(Some(gpsd_proto::Tpv {
                    time,
                    lat: Some(latitude),
                    lon: Some(longitude),
                    alt_hae: Some(alt),
                    ..
                })) => {
                    break Ok(Fix {
                        time: *time,
                        latitude: *latitude,
                        longitude: *longitude,
                        alt: *alt,
                    });
                }
                Ok(Some(_)) => {
                    // not all fields set yet
                }
            }
        }
    }
}

#[derive(Debug)]
enum Command {
    // nothing for now, but the channel is also used to terminate the receiver task
}

#[derive(Debug)]
struct GpsdReceiverTask {
    reader: Lines<BufReader<OwnedReadHalf>>,

    // note: we need to keep this around to not close the socket.
    #[allow(unused)]
    writer: BufWriter<OwnedWriteHalf>,

    command_receiver: mpsc::Receiver<Command>,
    fix_sender: watch::Sender<FixChannelItem>,
}

impl GpsdReceiverTask {
    async fn new(
        stream: TcpStream,
        command_receiver: mpsc::Receiver<Command>,
        fix_sender: watch::Sender<FixChannelItem>,
    ) -> Result<Self, Error> {
        let (reader, writer) = stream.into_split();

        let reader = BufReader::new(reader).lines();
        let mut writer = BufWriter::new(writer);

        writer.write_all(b"?WATCH=").await?;
        writer
            .write_all(
                &serde_json::to_vec(&gpsd_proto::Watch {
                    enable: true,
                    json: true,
                    ..Default::default()
                })
                .unwrap(),
            )
            .await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        Ok(Self {
            reader,
            writer,
            command_receiver,
            fix_sender,
        })
    }

    async fn run(mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
                command = self.command_receiver.recv() => {
                    let Some(command) = command else { break; };
                    self.handle_command(command).await?;
                }
                line = self.reader.next_line() => {
                    let Some(line) = line? else {
                        tracing::error!("gpsd connection closed");
                        break;
                    };
                    self.handle_message(line).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: Command) -> Result<(), Error> {
        match command {}
    }

    async fn handle_message(&mut self, json: String) -> Result<(), Error> {
        match serde_json::from_str::<gpsd_proto::Message>(&json) {
            Ok(message) => {
                //tracing::debug!(json, ?message, "gpsd message decoded");
                match message {
                    gpsd_proto::Message::Tpv(tpv) => {
                        //tracing::debug!(?tpv, %json);
                        let _ = self.fix_sender.send_replace(Ok(Some(tpv)));
                    }
                    gpsd_proto::Message::Error(error) => {
                        tracing::error!(%error.message);
                        todo!("handle error from server");
                    }
                    _ => {}
                }
            }
            Err(_error) => {
                //tracing::debug!(%error, json, "failed to decode gpsd
                // message");
            }
        }

        Ok(())
    }
}

mod gpsd_proto {
    #![allow(unused)]

    use chrono::{
        DateTime,
        Utc,
    };
    use serde::{
        Deserialize,
        Serialize,
    };

    #[derive(Clone, Debug, Deserialize)]
    #[serde(tag = "class", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum Message {
        Version(Version),
        Watch(Watch),
        Tpv(Tpv),
        Devices(Devices),
        Device(Device),
        Error(Error),
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Version {
        pub release: String,
        pub rev: String,
        pub proto_major: u32,
        pub proto_minor: u32,
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct Watch {
        pub enable: bool,
        pub json: bool,
        pub nmea: bool,
        pub raw: u8,
        pub scaled: bool,
        pub timing: bool,
        pub split24: bool,
        pub pps: bool,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Tpv {
        pub mode: TpvMode,

        pub time: DateTime<Utc>,

        /// Estimated time stamp error in seconds. Certainty unknown.
        pub ept: Option<f64>,

        /// Latitude in degrees: +/- signifies North/South.
        pub lat: Option<f64>,

        /// Longitude in degrees: +/- signifies East/West.
        pub lon: Option<f64>,

        /// Altitude, height above ellipsoid, in meters. Probably WGS84.
        #[serde(rename = "altHAE")]
        pub alt_hae: Option<f64>,

        /// MSL Altitude in meters. The geoid used is rarely specified and is
        /// often inaccurate.
        #[serde(rename = "altMSL")]
        pub alt_msl: Option<f64>,

        /// Longitude error estimate in meters. Certainty unknown.
        pub epx: Option<f64>,

        /// Latitude error estimate in meters. Certainty unknown.
        pub epy: Option<f64>,

        /// Estimated vertical error in meters. Certainty unknown.
        pub epv: Option<f64>,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(variant_identifier)]
    #[repr(u8)]
    pub enum TpvMode {
        Unknown = 0,
        NoFix = 1,
        Fix2d = 2,
        Fix3d = 3,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Devices {
        pub devices: Vec<Device>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Device {
        pub activated: Option<DateTime<Utc>>,
        pub path: Option<String>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Error {
        pub message: String,
    }
}
