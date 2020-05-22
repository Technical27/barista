use clap::{App, Arg};
use futures::{SinkExt, StreamExt};
use minelib::command::*;
use minelib::server::Status;
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::prelude::*;
use warp::ws::Message;
use warp::Filter;

mod config;
mod server;

use server::Server;

static WEBSITE_PATH: &'static str = "mineweb/dist";

struct State {
    servers: Vec<Server>,
}

impl State {
    pub fn new(config: config::Config) -> Self {
        let mut servers = vec![];
        for i in 0..config.servers.len() {
            let s = config.servers[i].clone();
            servers.push(Server::new(i as u64, s));
        }
        Self { servers }
    }
}

type GlobalState = Arc<Mutex<State>>;

fn handle_ws(ws: warp::ws::Ws, state: GlobalState) -> impl warp::Reply {
    let state = state.clone();
    ws.on_upgrade(|socket| async move {
        let (mut tx, mut rx) = socket.split();
        loop {
            if let Some(msg) = rx.next().await {
                match msg {
                    Ok(data) => {
                        if !data.is_binary() {
                            return;
                        }

                        let bytes = &data.as_bytes()[..];
                        let cmd = match serde_cbor::from_slice::<Command>(bytes) {
                            Ok(v) => v,
                            Err(e) => return eprintln!("error parsing command: {:?}", e),
                        };

                        let mut servers = {
                            let lock = state.lock().expect("failed to lock mutex");
                            lock.servers.clone().into_iter().map(|s| s.state).collect()
                        };

                        let res = match cmd {
                            Command::GetServers => CommandResult::UpdateServers(servers),
                            Command::StartServer(id) => {
                                servers[id as usize].status = Status::Starting;
                                CommandResult::UpdateServers(servers)
                            }
                        };

                        let msg =
                            serde_cbor::to_vec(&res).expect("failed to serialize ws response");

                        if let Err(e) = tx.send(Message::binary(msg)).await {
                            eprintln!("error sending ws message: {:?}", e);
                        }
                    }
                    Err(e) => eprintln!("error getting ws message: {:?}", e),
                }
            }
        }
    })
}

fn build_app() -> App<'static, 'static> {
    App::new("mined")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Aamaruvi Yogamani")
        .about("a daemon to control minecraft servers")
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .value_name("FILE")
                .help("Sets a custom config")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("website-path")
                .long("website-path")
                .value_name("DIR")
                .help("Sets the directory of the management website")
                .takes_value(true),
        )
}

#[tokio::main]
async fn main() {
    let matches = build_app().get_matches();

    let config = Path::new(matches.value_of("config").unwrap_or("/etc/mined/mined.yml"));

    let mut config_file = match File::open(config).await {
        Ok(f) => f,
        Err(e) => return eprintln!("failed to open config file: {}", e),
    };

    let mut config = vec![];
    if let Err(e) = config_file.read_to_end(&mut config).await {
        return eprintln!("failed to read config file: {}", e);
    }

    let config = match serde_yaml::from_slice::<config::Config>(&config) {
        Ok(c) => c,
        Err(e) => return eprintln!("failed to parse config: {}", e),
    };

    if config.version > 1 {
        return eprintln!("unsupported config version");
    }

    let state = Arc::new(Mutex::new(State::new(config)));
    let state = warp::any().map(move || state.clone());

    let path = env::current_dir()
        .expect("failed to get current directory")
        .join(matches.value_of("website-path").unwrap_or(WEBSITE_PATH));

    let idx = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file(path.join("index.html")));

    let dirs = warp::get().and(warp::fs::dir(path));

    let ws = warp::path("cmd")
        .and(warp::ws())
        .and(state.clone())
        .map(handle_ws);

    let routes = idx.or(dirs).or(ws);

    let addr = ([127, 0, 0, 1], 3000);
    warp::serve(routes).run(addr).await;
}
