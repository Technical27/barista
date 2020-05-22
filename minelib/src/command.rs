use super::server;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Command {
    GetServers,
    StartServer(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CommandResult {
    UpdateServers(Vec<server::State>),
}
