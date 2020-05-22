use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub id: u64,
    pub name: String,
    pub player_count: u64,
    pub status: Status,
}

impl State {
    pub fn new(id: u64, name: &str, player_count: u64) -> Self {
        Self {
            id,
            name: name.to_string(),
            player_count,
            status: Status::Stopped,
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
