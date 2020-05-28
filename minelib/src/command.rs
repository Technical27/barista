use super::server;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Command {
    GetServers,
    StartServer(usize),
    StopServer(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CommandResult {
    UpdateServers(Vec<server::State>),
    UpdateServer(usize, server::State),
}
