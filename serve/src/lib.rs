use cocoa::command::*;
use cocoa::config::Config;
use cocoa::server::ServerData;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use log::{error, info, trace, warn};
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::prelude::*;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use warp::ws::{Message, WebSocket};
use warp::Filter;

mod server;

use server::Server;

static WEBSITE_PATH: &'static str = "menu/dist";
static CONFIG_VERSION: u64 = 1;

type RuntimeClient = SplitSink<WebSocket, Message>;

#[derive(Debug)]
enum RuntimeMsg {
    Msg(Message),
    NewClient(RuntimeClient),
}

struct State {
    servers: Vec<Server>,
    tx: UnboundedSender<RuntimeMsg>,
}

impl State {
    pub fn new(config: Config, tx: UnboundedSender<RuntimeMsg>) -> Self {
        let mut servers = vec![];
        for id in 0..config.servers.len() {
            let cfg = config.servers[id].clone();
            let data = ServerData::new(id, cfg);
            servers.push(Server::new(data));
        }
        Self { servers, tx }
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

fn run_command(cmd: Command, state: GlobalState) -> CommandResult {
    let mut lock = state.lock()?;
    match cmd {
        Command::GetServers => {
            let server_data = lock.servers.iter().map(|s| s.data.clone()).collect();
            Ok(CommandResponse::UpdateServers(server_data))
        }
        Command::StartServer(id) => lock.servers[id].start(),
        Command::StopServer(id) => lock.servers[id].stop(),
    }
}

fn serve_ws(data: Message, state: GlobalState) -> Result<Message, WebsocketError> {
    if !data.is_binary() {
        return Err(WebsocketError::NotBinary);
    }

    let bytes = &data.as_bytes();
    let cmd = serde_cbor::from_slice::<Command>(bytes)?;

    let res = match run_command(cmd, state) {
        Ok(res) => res,
        Err(e) => {
            error!("error running command: {}", e);
            CommandResponse::Error(e)
        }
    };

    let msg = serde_cbor::to_vec(&res)?;

    Ok(Message::binary(msg))
}

fn handle_ws(ws: warp::ws::Ws, state: GlobalState) -> impl warp::Reply {
    let state = state.clone();
    ws.on_upgrade(|socket| async move {
        let (tx, mut rx) = socket.split();
        {
            let lock = state.lock().unwrap();
            lock.tx.send(RuntimeMsg::NewClient(tx)).ok();
        }
        loop {
            if let Some(m) = rx.next().await.map(|m| {
                m.map_err(|e| e.into())
                    .and_then(|msg| serve_ws(msg, state.clone()))
            }) {
                match m {
                    Ok(m) => {
                        let lock = state.lock().unwrap();
                        lock.tx.send(RuntimeMsg::Msg(m)).ok();
                    }
                    Err(WebsocketError::NotBinary) => trace!("ws message not binary, skipping"),
                    Err(e) => error!("{}", e),
                }
            } else {
                trace!("disconnect");
                break;
            }
        }
    })
}

async fn server_init(matches: Option<&clap::ArgMatches<'static>>) -> Result<(), ServerError> {
    let config = Path::new(
        matches
            .and_then(|c| c.value_of("config"))
            .unwrap_or("/etc/mined/mined.yml"),
    );

    let mut config_file = File::open(config).await?;

    let mut config = vec![];
    config_file.read_to_end(&mut config).await?;

    let config = serde_yaml::from_slice::<Config>(&config)?;

    if config.version > CONFIG_VERSION {
        return Err(ServerError::InvalidConfigVersion);
    } else if config.version < CONFIG_VERSION {
        warn!("current config is outdated, please update");
    }

    let (tx, mut rx) = unbounded_channel();

    let task = tokio::task::spawn(async move {
        let mut clients = vec![];
        loop {
            if let Some(cmd) = rx.recv().await {
                match cmd {
                    RuntimeMsg::NewClient(ws) => clients.push(ws),
                    RuntimeMsg::Msg(msg) => {
                        // this is the best solution I could think of to remove dead clients
                        // and its pretty bad
                        let mut dead_clients = vec![];
                        for (id, c) in &mut clients.iter_mut().enumerate() {
                            if let Err(_) = c.send(msg.clone()).await {
                                dead_clients.push(id);
                            }
                        }
                        for id in dead_clients {
                            let _ = clients.swap_remove(id);
                        }
                    }
                }
            }
        }
    });

    let state = Arc::new(Mutex::new(State::new(config, tx)));
    let state = warp::any().map(move || state.clone());

    let path = env::current_dir()
        .expect("failed to get current directory")
        .join(
            matches
                .and_then(|c| c.value_of("website-path"))
                .unwrap_or(WEBSITE_PATH),
        );

    let dirs = warp::get().and(warp::fs::dir(path.clone()));

    let idx = warp::get().and(warp::fs::file(path.join("index.html")));

    let ws = warp::path("cmd")
        .and(warp::ws())
        .and(state.clone())
        .map(handle_ws);

    let routes = dirs.or(ws).or(idx);

    let addr = ([127, 0, 0, 1], 3000);
    info!("starting server");
    tokio::join!(task, warp::serve(routes).run(addr)).0.unwrap();

    Ok(())
}

pub async fn serve(matches: Option<&clap::ArgMatches<'static>>) {
    pretty_env_logger::init_custom_env("MINED_LOG");

    server_init(matches)
        .await
        .map_err(|e| error!("{:?}", e))
        .ok();
}
