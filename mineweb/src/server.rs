use minelib::server::Server;
use yew::prelude::*;

#[derive(Properties, Clone)]
pub struct Props {
    pub server: Server,
}

pub struct ServerComponent {
    server: Server,
}

impl Component for ServerComponent {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {
            server: props.server,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        let eq = props.server != self.server;

        if eq {
            self.server = props.server;
        }

        eq
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let server = self.server.clone();
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
                <button class="server-btn">{
                    server.status
                }</button>
            </div>
        }
    }
}
