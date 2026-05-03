use std::{
    fs::File,
    io::{
        BufReader,
        BufWriter,
        Read,
        Seek,
        Write,
    },
    path::Path,
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
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    AntennaConfig,
    Frame,
};

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Invalid file signature")]
    InvalidSignature { invalid: [u8; 6] },

    #[error("Unsupported version: {version}")]
    UnsupportedVersion { version: u16 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileHeader {
    pub timestamp: DateTime<Utc>,
    pub config: AntennaConfig,
}

#[derive(Debug)]
pub struct FileReader {
    reader: BufReader<File>,
    version: u16,
    header: FileHeader,
}

impl FileReader {
    pub fn open(path: impl AsRef<Path>) -> Result<FileReader, ReadError> {
        let mut reader = BufReader::new(File::open(path)?);

        // read and verify file signature
        let mut signature = [0; 6];
        reader.read_exact(&mut signature)?;
        if &signature != b"STOLAS" {
            return Err(ReadError::InvalidSignature { invalid: signature });
        }

        // read and check version
        let version = reader.read_u16::<BigEndian>()?;
        if version != 0x02 {
            return Err(ReadError::UnsupportedVersion { version });
        }

        // read header
        let header_size = reader.read_u32::<BigEndian>()?;
        let mut header_json = vec![0; header_size as usize];
        reader.read_exact(&mut header_json)?;
        let header: FileHeader = serde_json::from_slice(&header_json)?;
        tracing::debug!(?header, "Header");

        Ok(FileReader {
            reader,
            version,
            header,
        })
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    pub fn read_chunk(&mut self) -> Result<Option<Chunk>, ReadError> {
        let tag = match self.reader.read_u8() {
            Ok(tag) => tag,
            Err(error) => {
                // try to read frame and handle end-of-file gracefully

                if error.kind() == std::io::ErrorKind::UnexpectedEof {
                    // end of file
                    return Ok(None);
                }
                else {
                    return Err(error.into());
                }
            }
        };

        let length = self.reader.read_u32::<BigEndian>()?;

        let body_start = self.reader.stream_position()?;
        let mut body_reader = (&mut self.reader).take(length.into());

        let chunk = match tag {
            CHUNK_FRAME => {
                let frame = Frame::read(body_reader)?;
                let invalid = frame.bins.len() != self.header.config.processing.window_size;
                Chunk::Frame { frame, invalid }
            }
            CHUNK_CONFIG => {
                let config = serde_json::from_reader(body_reader)?;
                Chunk::Config { config }
            }
            CHUNK_DROPPED => {
                let num_dropped = body_reader.read_u64::<BigEndian>()?;
                Chunk::Dropped { num_dropped }
            }
            _ => Chunk::Unrecognized { tag, length },
        };

        let remaining = (body_start + u64::from(length))
            .checked_sub(self.reader.stream_position()?)
            .expect("read too much data even though Take was used");
        if remaining > 0 {
            self.reader.seek_relative(remaining.try_into().unwrap())?;
        }

        Ok(Some(chunk))
    }

    /// Convenience method that skips all chunks that are not frames, or are
    /// invalid frames.
    pub fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
        loop {
            if let Some(chunk) = self.read_chunk()? {
                match chunk {
                    Chunk::Frame {
                        frame,
                        invalid: false,
                    } => {
                        return Ok(Some(frame));
                    }
                    _ => {}
                }
            }
            else {
                return Ok(None);
            }
        }
    }
}

pub const CHUNK_FRAME: u8 = b'F';
pub const CHUNK_CONFIG: u8 = b'C';
pub const CHUNK_DROPPED: u8 = b'D';

#[derive(Clone, Debug)]
pub enum Chunk {
    Frame { frame: Frame, invalid: bool },
    Config { config: AntennaConfig },
    Dropped { num_dropped: u64 },
    Unrecognized { tag: u8, length: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct FileWriter {
    writer: BufWriter<File>,
}

impl FileWriter {
    pub const VERSION: u16 = 0x0002;

    pub fn open(path: impl AsRef<Path>, header: &FileHeader) -> Result<Self, WriteError> {
        let path = path.as_ref();
        let timestamp = Utc::now();

        std::fs::create_dir_all(path)?;

        let file_path = path.join(format!("{}.rec", timestamp.to_rfc3339()));
        let mut writer = BufWriter::new(File::create_new(&file_path)?);

        let header_json = serde_json::to_vec(&header)?;
        writer.write_all(b"STOLAS")?;
        writer.write_u16::<BigEndian>(Self::VERSION)?;
        writer.write_u32::<BigEndian>(header_json.len().try_into().unwrap())?;
        writer.write_all(&header_json)?;

        Ok(Self { writer })
    }

    fn write_chunk_header(&mut self, tag: u8, length: u32) -> Result<(), WriteError> {
        self.writer.write_u8(tag)?;
        self.writer.write_u32::<BigEndian>(length)?;
        Ok(())
    }

    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), WriteError> {
        self.write_chunk_header(CHUNK_FRAME, frame.byte_length())?;
        frame.write(&mut self.writer)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn write_config(&mut self, config: &AntennaConfig) -> Result<(), WriteError> {
        let config_json = serde_json::to_vec(config)?;
        self.write_chunk_header(CHUNK_CONFIG, config_json.len().try_into().unwrap())?;
        self.writer.write_all(&config_json)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn write_dropped(&mut self, num_dropped: u64) -> Result<(), WriteError> {
        self.write_chunk_header(CHUNK_DROPPED, 8)?;
        self.writer.write_u64::<BigEndian>(num_dropped)?;
        self.writer.flush()?;
        Ok(())
    }
}
