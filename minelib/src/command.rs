use super::server::ServerData;
use serde::{Deserialize, Serialize};

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
}
