use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ServerConfig {
    pub name: String,
    pub jar: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub version: u64,
    pub servers: Vec<ServerConfig>,
}
