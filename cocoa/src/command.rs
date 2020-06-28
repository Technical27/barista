use super::server::ServerData;
use serde::{Deserialize, Serialize};
use std::sync::PoisonError;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Command {
    GetServers,
    StartServer(usize),
    StopServer(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CommandResponse {
    UpdateServers(Vec<ServerData>),
    UpdateServer(usize, ServerData),
    Error(CommandError),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CommandError {
    MutexLockFail,
    NonExistentServer(usize),
    SystemError(i32),
    UnknownSystemError,
}

pub type CommandResult = Result<CommandResponse, CommandError>;

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            Self::MutexLockFail => "failed to lock mutex".to_string(),
            Self::SystemError(e) => format!("system error: {}", e),
            Self::NonExistentServer(id) => format!("server id {} doesn't exist", id),
            Self::UnknownSystemError => "unknown system error".to_string(),
        };

        write!(f, "{}", msg)
    }
}

impl<T> From<PoisonError<T>> for CommandError {
    fn from(_: PoisonError<T>) -> Self {
        Self::MutexLockFail
    }
}

#[cfg(unix)]
impl From<nix::Error> for CommandError {
    fn from(e: nix::Error) -> Self {
        match e {
            nix::Error::Sys(e) => Self::SystemError(e as i32),
            _ => Self::UnknownSystemError,
        }
    }
}
