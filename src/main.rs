use std::env;

use futures::stream::StreamExt;
use telegram_bot::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    let api = Api::new(token);

    // Fetch new updates via long poll method
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        if let UpdateKind::Message(ref message) = update.expect("fuck").kind {
            if let MessageKind::Text { ref data, .. } = message.kind {
                // Print received text message to stdout.
                println!("<{}>: {}", &message.from.first_name, data);

                // Answer message with "Hi".
                let username = message.from.username.as_ref().unwrap();
                api.send(
                    message
                        .text_reply(format!("Hi, @{}! You just wrote '{}'", username, data))
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            }
        }
    }
    Ok(())
}
