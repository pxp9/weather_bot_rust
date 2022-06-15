mod json_parse;
use crate::json_parse::*;
use serde_json::Value;

use futures::stream::StreamExt;
use std::env;
use telegram_bot::Error;
use telegram_bot::*;

// only use if response is Text
async fn get_message_data(update: Update) -> Result<(Message, String), ()> {
    if let UpdateKind::Message(message2) = update.kind {
        if let MessageKind::Text { ref data, .. } = message2.kind {
            return Ok((message2.clone(), data.clone()));
        }
        return Err(());
    }
    return Err(());
}
async fn city(
    api: &Api,
    message: Message,
    stream: &mut UpdatesStream,
    cities: &Vec<Value>,
) -> Result<(), Error> {
    let user = match message.from.username.as_ref() {
        Some(username) => username.clone(),
        None => message.from.first_name.clone(),
    };
    api.send(
        message
            .text_reply(format!(
                "Hi, @{}! Write city and country acronym like this Madrid,ES",
                user
            ))
            .parse_mode(ParseMode::Markdown),
    )
    .await?;
    // bot espera a una respuesta del cliente
    if let Some(update) = stream.next().await {
        let update = update?;
        let (message2, data): (Message, String) = get_message_data(update)
            .await
            .expect("lo llamaste con algo que no era texto");
        let v: Vec<&str> = data.split(",").collect();
        let city = v[0];
        let country = v[1];
        let (lon, lat) =
            match search_city((*city).to_string(), (*country).to_string(), cities).await {
                Ok((lon, lat)) => (lon, lat),
                Err(_) => (-181.0, -91.0),
            };
        if lat == -91.0 {
            api.send(
                message2
                    .text_reply(format!("User @{} ,  City {} not found", user, city))
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        } else {
            // spanish sp and english en
            // units metric or imperial
            let request_url = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units={}&lang={}",
            lat,
            lon,
            env::var("OPEN_WEATHER_MAP_API_TOKEN").expect("OPEN_WEATHER_MAP_API_TOKEN not set"),
            "metric",
            "sp",
        );
            let response = reqwest::get(&request_url).unwrap().text().unwrap();
            let weather_info = parse_weather(response).await.unwrap();
            api.send(
                message2
                    .text_reply(format!(
                        "User @{} ,  City {} , Country {} , Lon {} , Lat {}  Weather {}",
                        user, city, country, lon, lat, weather_info,
                    ))
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        }
    }
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    let json = read_json_cities();
    let api = Api::new(token);

    // Fetch new updates via long poll method
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        match update.kind {
            UpdateKind::Message(message) => match message.kind {
                MessageKind::Text { ref data, .. } => match data.as_str() {
                    "/city" => {
                        city(&api, message, &mut stream, &json).await?;
                    }
                    _ => (),
                    // Print received text message to stdout.
                    //println!("<{}>: {}", &message.from.first_name, data);

                    // Answer message with "Hi".
                    /* let username = message.from.username.as_ref().unwrap();
                    api.send(
                        message
                            .text_reply(format!("Hi, @{}! You just wrote '{}'", username, data))
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;*/
                },
                _ => (),
            },
            _ => (),
        }
    }
    Ok(())
}
