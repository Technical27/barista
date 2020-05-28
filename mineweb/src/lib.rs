use minelib::command::*;
use minelib::server::Server;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use yew::prelude::*;

mod server;
use server::ServerComponent;

struct Model {
    link: ComponentLink<Self>,
    server_list: Vec<Server>,
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

    fn format_server(&self, server: &Server) -> Html {
        html! { <ServerComponent server={server} /> }
    }
}

enum Msg {
    Websocket(CommandResult),
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
