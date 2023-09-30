use crate::AgentError::LockError;
use serde::{Deserialize, Serialize};
use std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum AgentError {
    #[error("Invalid environment ID")]
    InvalidEnvironmentId,
    #[error("IO Error")]
    IoError,
    #[error("Invalid file ID")]
    InvalidFileDescriptor,
    #[error("Invalid seek whence")]
    InvalidSeekWhence,
    #[error("Lock Error")]
    LockError,
    #[error("Failed to start process")]
    ProcessStartFailure,
    #[error("Invalid process ID")]
    InvalidProcessId,
    #[error("Process channel not piped")]
    ProcessChannelNotPiped,
    #[error("The server state is inconsistent")]
    Inconsistent,
    #[error("Unknown Error")]
    Unknown,
}

impl<T> From<PoisonError<T>> for AgentError {
    fn from(_: PoisonError<T>) -> Self {
        LockError
    }
}

impl<T> From<RwLockReadGuard<'_, T>> for AgentError {
    fn from(_: RwLockReadGuard<T>) -> Self {
        LockError
    }
}

impl<T> From<RwLockWriteGuard<'_, T>> for AgentError {
    fn from(_: RwLockWriteGuard<T>) -> Self {
        LockError
    }
}
