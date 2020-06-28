use log::error;
use cocoa::command::*;
use cocoa::server::{ServerData, Status};
use web_sys::WebSocket;
use yew::prelude::*;

macro_rules! neq_assign {
    (($self:ident, $props:ident) => $e:ident) => {
        if $self.$e != $props.$e { $self.$e = $props.$e; true } else { false }
    };
    (($self:ident, $props:ident) => $( $e:ident ), +) => {
        $(neq_assign!(($self, $props) => $e))||+
    };
}

#[derive(Properties, Clone)]
pub struct Props {
    pub servers: Vec<ServerData>,
    pub ws: WebSocket,
}

pub enum Msg {
    SendWebsocket(Command),
    None,
}

pub struct ServerPage {
    servers: Vec<ServerData>,
    ws: WebSocket,
    link: ComponentLink<Self>,
}

impl ServerPage {
    fn send_ws(&self, cmd: Command) {
        if let Err(e) = serde_cbor::to_vec(&cmd)
            .map_err(|e| e.to_string().into())
            .and_then(|arr| self.ws.send_with_u8_array(&arr))
        {
            error!("failed to send ws message: {:?}", e);
        }
    }

    fn handle_button(server: &ServerData) -> Msg {
        let cmd = match server.status {
            Status::Open => Command::StopServer(server.id),
            Status::Stopped => Command::StartServer(server.id),
            _ => return Msg::None,
        };

        Msg::SendWebsocket(cmd)
    }

    fn format_server(&self, server: &ServerData) -> Html {
        let server = server.clone();
        let s = server.clone();
        html! {
            <div class="server">
                <span class="server-name">{
                    server.name.clone()
                }</span>
                <span class="server-player-count">{
                    format!("Player Count: {}", server.player_count)
                }</span>
                <span class="server-status">{
                    format!("Status: {}", server.status)
                }</span>
                <button class="server-btn" onclick=self.link.callback(move |_| Self::handle_button(&s))>{
                    server.status
                }</button>
            </div>
        }
    }
}

impl Component for ServerPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            servers: props.servers,
            ws: props.ws,
            link,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        neq_assign!((self, props) => ws, servers)
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SendWebsocket(cmd) => self.send_ws(cmd),
            Msg::None => return false,
        }

        true
    }

    fn view(&self) -> Html {
        html! {
            { for self.servers.iter().map(|s| self.format_server(s)) }
        }
    }
}
