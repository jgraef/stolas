use std::process::{
    ExitStatus,
    Output,
};

use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
#[error("program exited with status: {exit_status}")]
pub struct ExitStatusError {
    exit_status: ExitStatus,
}

pub trait ExitStatusExt {
    fn into_result(self) -> Result<(), ExitStatusError>;
}

impl ExitStatusExt for ExitStatus {
    fn into_result(self) -> Result<(), ExitStatusError> {
        if self.success() {
            Ok(())
        }
        else {
            Err(ExitStatusError { exit_status: self })
        }
    }
}

pub trait OutputExt: Sized {
    fn into_result(self) -> Result<Self, ExitStatusError>;
}

impl OutputExt for Output {
    fn into_result(self) -> Result<Self, ExitStatusError> {
        self.status.into_result()?;
        Ok(self)
    }
}

pub trait OutputJsonExt: OutputExt {
    fn into_json_result<T: DeserializeOwned>(self) -> Result<T, OutputJsonError>;
}

impl OutputJsonExt for Output {
    fn into_json_result<T: DeserializeOwned>(self) -> Result<T, OutputJsonError> {
        let output = self.into_result()?;
        let value: T = serde_json::from_slice(&output.stdout)?;
        Ok(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OutputJsonError {
    #[error(transparent)]
    ExitStatus(#[from] ExitStatusError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
