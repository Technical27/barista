use minelib::prelude::*;

#[get("/servers")]
pub fn get_servers() -> Vec<Server> {
    vec![Server::new("test", 69)]
}
