extern crate alectro;
extern crate futures;
extern crate irc;
extern crate termion;

use tokio_stream::StreamExt;
use alectro::controller::{InputController, IrcController};
use alectro::input::AsyncKeyInput;
use alectro::view::UI;
use alectro::model::Event;
use irc::client::prelude::*;

#[tokio::main]
async fn main() {
    let ui = UI::new().unwrap();

    let default_cfg = Config {
        nickname: Some(format!("aatxe")),
        server: Some(format!("chat.freenode.net")),
        use_tls: Some(true),
        .. Default::default()
    };

    let cfg = match dirs::home_dir() {
        Some(mut path) => {
            path.push(".alectro");
            path.set_extension("toml");
            Config::load(path).unwrap_or(default_cfg)
        },
        None => default_cfg,
    };

    for chan in cfg.channels() {
        ui.new_chat_buf(chan).unwrap();
    }

    let mut irc_client = Client::from_config(cfg).await.expect("Could not create IRC client");
    irc_client.identify().unwrap();
    let mut stream = irc_client.stream().expect("Could not get stream from client");

    let irc_controller = IrcController::new(ui.clone());

    let input_controller = InputController::new(irc_client, ui);
    let mut input_rx = AsyncKeyInput::new();
    input_controller.ui().add_event_to_current_chat_buf(
        Event::notice(None, "LOG", "spawned input handler thread")
    ).expect("Could not add log message to current chat buffer");
    tokio::spawn(async move {
        while let Some(event) = input_rx.next().await {
            input_controller.handle_event(event).expect("Could not handle event");
            input_controller.ui().draw_all().expect("Could not draw UI");
        }
    });

    while let Some(message) = stream.next().await.transpose().expect("Could not receive message") {
        irc_controller.handle_message(message).expect("Could not handle message");
        irc_controller.ui().draw_all().expect("Could not draw UI");
    }

}
