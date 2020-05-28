use super::server::Server;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Command {
    GetServers,
    StartServer(usize),
    StopServer(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CommandResult {
    UpdateServers(Vec<Server>),
    UpdateServer(usize, Server),
}
