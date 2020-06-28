use log::{error, trace};
use cocoa::command::*;
use cocoa::server::ServerData;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use yew::prelude::*;
use yew_router::components::RouterAnchor;
use yew_router::router::Router;
use yew_router::Switch;

mod server;
use server::ServerPage;

#[derive(Switch, Debug, Clone, Copy)]
enum AppRoute {
    #[to = "/"]
    Index,
}

type AppLink = (&'static str, AppRoute);

struct App {
    link: ComponentLink<Self>,
    server_list: Vec<ServerData>,
    ws: WebSocket,
    nav_items: Vec<AppLink>,
}

impl App {
    fn init_websocket(&self) {
        let ws = self.ws.clone();
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let link = self.link.clone();
        let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            match e.data().dyn_into::<js_sys::ArrayBuffer>().and_then(|abuf| {
                let data = js_sys::Uint8Array::new(&abuf).to_vec();
                serde_cbor::from_slice::<CommandResponse>(&data).map_err(|e| e.to_string().into())
            }) {
                Ok(msg) => {
                    trace!("new message: {:?}", msg);
                    link.send_message(Msg::Websocket(msg));
                }
                Err(e) => error!("failed to get message: {:?}", e),
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();

        let ws_clone = ws.clone();
        let callback = Closure::wrap(Box::new(move |_| {
            if let Err(e) = serde_cbor::to_vec(&Command::GetServers)
                .map_err(|e| e.to_string().into())
                .and_then(|arr| ws_clone.send_with_u8_array(&arr))
            {
                error!("failed to get servers: {:?}", e)
            }
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn gen_link(link: &AppLink) -> Html {
        html! {
            <RouterAnchor<AppRoute> route={link.1} classes="nav-item">
                { link.0.clone() }
            </RouterAnchor<AppRoute>>
        }
    }
}

enum Msg {
    Websocket(CommandResponse),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let s = Self {
            link,
            server_list: vec![],
            ws: WebSocket::new("ws://localhost:3000/cmd").unwrap(),
            nav_items: vec![("Servers", AppRoute::Index)],
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
                CommandResponse::UpdateServers(servers) => self.server_list = servers,
                CommandResponse::UpdateServer(id, server) => self.server_list[id] = server,
                CommandResponse::Error(e) => error!("{}", e),
            },
        }

        true
    }

    fn view(&self) -> Html {
        let s = self.server_list.clone();
        let ws = self.ws.clone();
        let routes = Router::<AppRoute, ()>::render(move |sw: AppRoute| match sw {
            AppRoute::Index => html! {
                <ServerPage servers={s.clone()} ws={ws.clone()}/>
            },
        });
        html! {
            <div id="app">
                <div id="nav-bar">
                    <span id="nav-title">{ "Menu" }</span>
                    { for self.nav_items.iter().map(Self::gen_link) }
                </div>
                <div id="page">
                    <Router<AppRoute, ()> render={routes} />
                </div>
            </div>
        }
    }
}

#[wasm_bindgen]
pub fn run() {
    web_logger::init();
    yew::start_app::<App>();
}
