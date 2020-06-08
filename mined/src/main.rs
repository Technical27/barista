use clap::{App, Arg};
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use minelib::command::*;
use minelib::config::Config;
use minelib::server::{Server, Status};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::prelude::*;
use warp::ws::Message;
use warp::Filter;

static WEBSITE_PATH: &'static str = "mineweb/dist";
static CONFIG_VERSION: u64 = 1;

struct State {
    servers: Vec<Server>,
    processes: HashMap<usize, process::Child>,
}

impl State {
    pub fn new(config: Config) -> Self {
        let mut servers = vec![];
        for i in 0..config.servers.len() {
            let s = config.servers[i].clone();
            servers.push(Server::new(i, s));
        }
        Self {
            servers,
            processes: HashMap::new(),
        }
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

                        let bytes = &data.as_bytes();
                        let cmd = match serde_cbor::from_slice::<Command>(bytes) {
                            Ok(v) => v,
                            Err(e) => return error!("error parsing command: {:?}", e),
                        };

                        let res = match cmd {
                            Command::GetServers => {
                                CommandResult::UpdateServers(state.lock().unwrap().servers.clone())
                            }
                            Command::StartServer(id) => {
                                let mut lock = state.lock().unwrap();
                                let server = &mut lock.servers[id].clone();
                                let cfg = server.config.clone();
                                let dir = Path::new(&cfg.dir);
                                let jar = dir.join(cfg.jar);
                                let mut args = vec![
                                    "-jar".to_string(),
                                    jar.to_str().unwrap().to_string(),
                                    "nogui".to_string(),
                                ];
                                args.append(&mut cfg.args.clone());
                                let child = process::Command::new("java")
                                    .args(args)
                                    .current_dir(dir)
                                    .spawn()
                                    .unwrap();
                                lock.processes.insert(id, child);
                                server.status = Status::Open;
                                CommandResult::UpdateServer(id, server.clone())
                            }
                            Command::StopServer(id) => {
                                let mut lock = state.lock().unwrap();
                                let server = &mut lock.servers[id].clone();
                                lock.processes.get_mut(&id).unwrap().kill().unwrap();
                                server.status = Status::Stopped;
                                CommandResult::UpdateServer(id, server.clone())
                            }
                        };

                        let msg = match serde_cbor::to_vec(&res) {
                            Ok(m) => m,
                            Err(e) => {
                                return error!("failed to serialize command response: {:?}", e)
                            }
                        };

                        if let Err(e) = tx.send(Message::binary(msg)).await {
                            error!("error sending ws message: {:?}", e);
                        }
                    }
                    Err(e) => error!("error getting ws message: {:?}", e),
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
    pretty_env_logger::init_custom_env("MINED_LOG");

    let matches = build_app().get_matches();

    let config = Path::new(matches.value_of("config").unwrap_or("/etc/mined/mined.yml"));

    let mut config_file = match File::open(config).await {
        Ok(f) => f,
        Err(e) => return error!("failed to open config file: {}", e),
    };

    let mut config = vec![];
    if let Err(e) = config_file.read_to_end(&mut config).await {
        return error!("failed to read config file: {}", e);
    }

    let config = match serde_yaml::from_slice::<Config>(&config) {
        Ok(c) => c,
        Err(e) => return error!("failed to parse config: {}", e),
    };

    if config.version > CONFIG_VERSION {
        return error!("unsupported config version");
    } else if config.version < CONFIG_VERSION {
        warn!("current config is outdated, please update");
    }

    let state = Arc::new(Mutex::new(State::new(config)));
    let state = warp::any().map(move || state.clone());

    let path = env::current_dir()
        .expect("failed to get current directory")
        .join(matches.value_of("website-path").unwrap_or(WEBSITE_PATH));

    let dirs = warp::get().and(warp::fs::dir(path.clone()));

    let idx = warp::get().and(warp::fs::file(path.join("index.html")));

    let ws = warp::path("cmd")
        .and(warp::ws())
        .and(state.clone())
        .map(handle_ws);

    let routes = dirs.or(ws).or(idx);

    let addr = ([127, 0, 0, 1], 3000);
    info!("starting server");
    warp::serve(routes).run(addr).await;
}
