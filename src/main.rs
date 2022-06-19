mod json_parse;
use crate::json_parse::*;
use futures::stream::StreamExt;
use std::collections::HashMap;
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
    conf : Conf<'_>
) -> Result<(), Error> {
    let user = match conf.message.from.username.as_ref() {
        Some(username) => format!("@{}", username.clone()),
        None => conf.message.from.first_name.clone(),
    };
    conf.api.send(
        conf.message
            .text_reply(format!(
                "Hi, {}! Write city and country acronym like this Madrid,ES",
                user
            ))
            .parse_mode(ParseMode::Markdown),
    )
    .await?;
    // bot espera a una respuesta del cliente
    if let Some(update) = conf.stream.next().await {
        let update = update?;
        let (message2, data): (Message, String) = get_message_data(update)
            .await
            .expect("lo llamaste con algo que no era texto");
        let v: Vec<&str> = data.split(",").collect();
        if v.len() < 2 {
            conf.api.send(
                conf.message
                    .text_reply(format!(
                        "Hi, {}! Write it in the correct format please {}",
                        user, data
                    ))
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
            return Ok(());
        }
        let city = v[0].trim();
        let country = v[1].trim();

        let (lon, lat, city_fmt, country_fmt) =
            match search_city((*city).to_string(), (*country).to_string(), conf.cities).await {
                Ok((lon, lat, city_fmt, country_fmt)) => (lon, lat, city_fmt, country_fmt),
                Err(_) => (-181.0, -91.0, String::from(""), String::from("")),
            };
        if lat == -91.0 {
            println!("User {} ,  City {} not found", user, city);
            conf.api.send(
                message2
                    .text_reply(format!("User {} ,  City {} not found", user, city))
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
            conf.opwm_token, 
            "metric",
            "sp",
        );
            let response = reqwest::get(&request_url).unwrap().text().unwrap();
            let weather_info = parse_weather(response).await.unwrap();
            println!(
                "User {} ,  City {} , Country {}\nLon {} , Lat {} {}",
                user, city_fmt, country_fmt, lon, lat, weather_info
            );
            conf.api.send(
                message2
                    .text_reply(format!(
                        "User {} ,  City {} , Country {}\nLon {} , Lat {}{}",
                        user, city_fmt, country_fmt, lon, lat, weather_info,
                    ))
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        }
    }

    Ok(())
}
struct Conf<'a> {
    api: &'a Api,
    message : Message ,
    cities: &'a HashMap<(String, String), (f64, f64, String, String)>,
    stream: &'a mut UpdatesStream,
    opwm_token : &'a String ,
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    let json = read_json_cities();
    let api = Api::new(token);
    let opwm_token = env::var("OPEN_WEATHER_MAP_API_TOKEN").expect("OPEN_WEATHER_MAP_API_TOKEN not set");
    // Fetch new updates via long poll method
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        match update.kind {
            UpdateKind::Message(message) => match message.kind {
                MessageKind::Text { ref data, .. } => match data.as_str() {
                    "/city" => {
                        let conf = Conf 
                        { api: &api, message : message , cities : &json, stream : &mut stream , opwm_token :&opwm_token};
                        city(conf).await?;
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
    }
    Ok(())
}
