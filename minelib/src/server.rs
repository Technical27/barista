use super::config::ServerConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub id: usize,
    pub name: String,
    pub player_count: u64,
    pub status: Status,
    pub config: ServerConfig,
}

impl Server {
    pub fn new(id: usize, config: ServerConfig) -> Self {
        Self {
            id,
            name: config.name.clone(),
            player_count: 0,
            status: Status::Stopped,
            config,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Status {
    Open,
    Starting,
    Stopping,
    Stopped,
    Crashed,
}

impl Default for Status {
    fn default() -> Status {
        Status::Stopped
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            Status::Open => "Open",
            Status::Starting => "Starting",
            Status::Stopping => "Stopping",
            Status::Stopped => "Stopped",
            Status::Crashed => "Crashed",
        };

        write!(f, "{}", msg)
    }
}
