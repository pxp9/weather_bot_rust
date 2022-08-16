use weather_bot_rust::telegram::handler::Handler;
use weather_bot_rust::workers;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    workers::start_workers().await;

    let mut handler = Handler::new().await;

    handler.start().await;
}
