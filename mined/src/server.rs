use minelib::server::ServerData;
use std::process::Child;

#[derive(Debug)]
pub struct Server {
    pub data: ServerData,
    process: Option<Child>,
}

impl Server {
    pub fn new(data: ServerData) -> Self {
        Self {
            data,
            process: None,
        }
    }
}
