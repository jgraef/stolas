use std::{
    fs::File,
    io::{
        BufReader,
        BufWriter,
        Read,
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
    InvalidSignature { invalid: [u8; 8] },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileHeader {
    pub timestamp: DateTime<Utc>,
    pub config: AntennaConfig,
}

#[derive(Debug)]
pub struct FileReader {
    reader: BufReader<File>,
    header: FileHeader,
}

impl FileReader {
    pub fn open(path: impl AsRef<Path>) -> Result<FileReader, ReadError> {
        let mut reader = BufReader::new(File::open(path)?);

        // read and verify file signature
        let mut signature = [0; 8];
        reader.read_exact(&mut signature)?;
        if &signature != b"STOLAS\x00\x01" {
            return Err(ReadError::InvalidSignature { invalid: signature });
        }

        // read header
        let header_size = reader.read_u32::<BigEndian>()?;
        let mut header_json = vec![0; header_size as usize];
        reader.read_exact(&mut header_json)?;
        let header: FileHeader = serde_json::from_slice(&header_json)?;
        tracing::debug!(?header, "Header");

        Ok(FileReader { reader, header })
    }

    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    pub fn read_frame(&mut self) -> Result<Option<Frame>, ReadError> {
        match Frame::read(&mut self.reader) {
            Ok(frame) => {
                assert_eq!(frame.bins.len(), self.header.config.processing.window_size);
                Ok(Some(frame))
            }
            Err(error) => {
                // try to read frame and handle end-of-file gracefully

                if error.kind() == std::io::ErrorKind::UnexpectedEof {
                    // end of file
                    Ok(None)
                }
                else {
                    Err(error.into())
                }
            }
        }
    }
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
    pub fn open(path: impl AsRef<Path>, header: &FileHeader) -> Result<Self, WriteError> {
        let path = path.as_ref();
        let timestamp = Utc::now();

        std::fs::create_dir_all(path)?;

        let file_path = path.join(format!("{}.rec", timestamp.to_rfc3339()));
        let mut writer = BufWriter::new(File::create_new(&file_path)?);

        let header_json = serde_json::to_string(&header)?;
        writer.write_all(b"STOLAS\x00\x01")?;
        writer.write_u32::<BigEndian>(header_json.len().try_into().unwrap())?;
        writer.write_all(header_json.as_bytes())?;

        Ok(Self { writer })
    }

    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), WriteError> {
        frame.write(&mut self.writer)?;
        self.writer.flush()?;
        Ok(())
    }
}
