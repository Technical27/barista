use clap::{App, Arg};
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use minelib::command::*;
use minelib::config::Config;
use minelib::server::{ServerData, Status};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::{self, Child};
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::prelude::*;
use warp::ws::Message;
use warp::Filter;

mod server;

use server::Server;

static WEBSITE_PATH: &'static str = "mineweb/dist";
static CONFIG_VERSION: u64 = 1;

struct State {
    servers: Vec<Server>,
    processes: HashMap<usize, Child>,
}

impl State {
    pub fn new(config: Config) -> Self {
        let mut servers = vec![];
        for id in 0..config.servers.len() {
            let cfg = config.servers[id].clone();
            let data = ServerData::new(id, cfg);
            servers.push(Server::new(data));
        }
        Self {
            servers,
            processes: HashMap::new(),
        }
    }
}

type GlobalState = Arc<Mutex<State>>;

#[derive(Debug)]
enum WebsocketError {
    NotBinary,
    ParseError(serde_cbor::Error),
    WarpError(warp::Error),
}

impl From<warp::Error> for WebsocketError {
    fn from(e: warp::Error) -> Self {
        Self::WarpError(e)
    }
}

impl From<serde_cbor::Error> for WebsocketError {
    fn from(e: serde_cbor::Error) -> Self {
        Self::ParseError(e)
    }
}

#[derive(Debug)]
enum ServerError {
    InvalidConfig(serde_yaml::Error),
    InvalidConfigVersion,
    IoError(std::io::Error),
}

impl From<serde_yaml::Error> for ServerError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::InvalidConfig(e)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl std::fmt::Display for WebsocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            Self::NotBinary => "not a binary websocket message".to_string(),
            Self::ParseError(e) => format!("failed to parse/serialize websocket message: {}", e),
            Self::WarpError(e) => format!("server error: {}", e),
        };

        write!(f, "{}", msg)
    }
}

impl std::error::Error for WebsocketError {}

#[cfg(unix)]
fn kill_child(child: Child) -> Result<(), nix::Error> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM)
}

#[cfg(windows)]
fn kill_child(child: process::Child) {
    use winapi::um::winuser::{EnumWindows, GetWindowThreadProcessId};
    child.kill().unwrap();
}

fn serve_ws(data: Message, state: GlobalState) -> Result<Message, WebsocketError> {
    if !data.is_binary() {
        return Err(WebsocketError::NotBinary);
    }

    let bytes = &data.as_bytes();
    let cmd = serde_cbor::from_slice::<Command>(bytes)?;

    let res = match cmd {
        Command::GetServers => {
            let lock = state.lock().unwrap();
            let server_data = lock.servers.iter().map(|s| s.data.clone()).collect();
            CommandResponse::UpdateServers(server_data)
        }
        Command::StartServer(id) => {
            let mut lock = state.lock().unwrap();
            let cfg = {
                let s = &mut lock.servers[id];
                s.data.config.clone()
            };
            let dir = Path::new(&cfg.dir);
            let jar = dir.join(cfg.jar);
            let mut args = vec![
                "-jar".to_string(),
                jar.to_str().unwrap().to_string(),
                "nogui".to_string(),
            ];
            args.append(&mut cfg.args.clone());
            let res = process::Command::new("java")
                .args(args)
                .current_dir(dir)
                .spawn();

            let data = {
                let mut s = {
                    let s = &mut lock.servers[id].data;
                    s.clone()
                };
                s.status = match res {
                    Ok(child) => {
                        lock.processes.insert(id, child);
                        Status::Open
                    }
                    Err(e) => {
                        error!("failed to start server: {}", e);
                        Status::Crashed
                    }
                };
                s
            };

            CommandResponse::UpdateServer(id, data.clone())
        }
        Command::StopServer(id) => {
            let mut lock = state.lock().unwrap();
            let server = &mut lock.servers[id].data.clone();
            let child = lock.processes.remove(&id).unwrap();
            kill_child(child).unwrap();
            server.status = Status::Stopped;
            CommandResponse::UpdateServer(id, server.clone())
        }
    };

    let msg = serde_cbor::to_vec(&res)?;

    Ok(Message::binary(msg))
}

fn handle_ws(ws: warp::ws::Ws, state: GlobalState) -> impl warp::Reply {
    let state = state.clone();
    ws.on_upgrade(|socket| async move {
        let (mut tx, mut rx) = socket.split();
        loop {
            if let Some(m) = rx.next().await.map(|m| {
                m.map_err(|e| e.into())
                    .and_then(|msg| serve_ws(msg, state.clone()))
            }) {
                match m {
                    Ok(m) => {
                        if let Err(e) = tx.send(m).await {
                            error!("failed to send ws message: {:?}", e);
                        }
                    }
                    Err(WebsocketError::NotBinary) => {}
                    Err(e) => error!("{}", e),
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

async fn server_init(matches: clap::ArgMatches<'static>) -> Result<(), ServerError> {
    let config = Path::new(matches.value_of("config").unwrap_or("/etc/mined/mined.yml"));

    let mut config_file = File::open(config).await?;

    let mut config = vec![];
    config_file.read_to_end(&mut config).await?;

    let config = serde_yaml::from_slice::<Config>(&config)?;

    if config.version > CONFIG_VERSION {
        return Err(ServerError::InvalidConfigVersion);
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

    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init_custom_env("MINED_LOG");

    let matches = build_app().get_matches();

    server_init(matches)
        .await
        .map_err(|e| error!("{:?}", e))
        .ok();
}
