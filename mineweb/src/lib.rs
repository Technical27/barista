use minelib::command::*;
use minelib::server;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use yew::prelude::*;

struct Model {
    link: ComponentLink<Self>,
    server_list: Vec<server::State>,
    ws: WebSocket,
}

impl Model {
    fn init_websocket(&self) {
        let ws = self.ws.clone();
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let lnk = self.link.clone();
        let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let data = js_sys::Uint8Array::new(&abuf).to_vec();
                if let Ok(msg) = serde_cbor::from_slice::<CommandResult>(&data[..]) {
                    lnk.send_message(Msg::Websocket(msg));
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();

        let ws_clone = ws.clone();
        let callback =
            Closure::wrap(
                Box::new(move |_| match serde_cbor::to_vec(&Command::GetServers) {
                    Ok(arr) => ws_clone.send_with_u8_array(&arr).unwrap(),
                    Err(_) => {}
                }) as Box<dyn FnMut(JsValue)>,
            );
        ws.set_onopen(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn handle_button(server: &server::State) -> Msg {
        use server::Status;
        match server.status {
            Status::Open => Msg::StopServer(server.id),
            Status::Stopped => Msg::StartServer(server.id),
            _ => Msg::None,
        }
    }

    fn format_server(&self, server: &server::State) -> Html {
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

    fn send_ws(&self, cmd: &Command) {
        let data = serde_cbor::to_vec(&cmd).unwrap();
        self.ws.send_with_u8_array(&data).unwrap();
    }
}

enum Msg {
    Websocket(CommandResult),
    StartServer(usize),
    StopServer(usize),
    None,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let s = Self {
            link,
            server_list: vec![],
            ws: WebSocket::new("ws://localhost:3000/cmd").unwrap(),
        };
        s.init_websocket();
        s
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Websocket(res) => match res {
                CommandResult::UpdateServers(servers) => self.server_list = servers,
                CommandResult::UpdateServer(idx, server) => self.server_list[idx] = server,
            },
            Msg::StartServer(id) => self.send_ws(&Command::StartServer(id)),
            Msg::StopServer(id) => self.send_ws(&Command::StopServer(id)),
            Msg::None => return false,
        }

        true
    }

    fn view(&self) -> Html {
        html! {
            <>
                <h1 id="title">{ "Minecraft Server Manager" }</h1>
                <h2 class="subtitle">{ "Active Servers" }</h2>
                <div id="server-list">
                    { for self.server_list.iter().map(|s| self.format_server(s)) }
                </div>
            </>
        }
    }
}

#[wasm_bindgen]
pub fn run() {
    App::<Model>::new().mount_to_body();
}
