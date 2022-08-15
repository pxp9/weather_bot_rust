use weather_bot_rust::json_parse;
use weather_bot_rust::telegram::handler::Handler;
use weather_bot_rust::workers;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    json_parse::read_json_cities().await.unwrap();

    workers::start_workers().await;

    let mut handler = Handler::new().await;

    handler.start().await;
}
