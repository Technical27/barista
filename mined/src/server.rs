use super::config::ServerConfig;
use minelib::server;

#[derive(Debug, Clone)]
pub struct Server {
    pub state: server::State,
    pub config: ServerConfig,
}

impl Server {
    pub fn new(id: usize, config: ServerConfig) -> Self {
        Self {
            state: server::State::new(id, &config.name, 0),
            config,
        }
    }
}
