mod json_parse;
use crate::json_parse::*;
mod database_manage;
use crate::database_manage::*;
use frankenstein::api_params::{ChatAction, SendChatActionParams};
use frankenstein::AsyncTelegramApi;
use frankenstein::Error;
use frankenstein::GetUpdatesParams;
use frankenstein::Message;
use frankenstein::ParseMode;
use frankenstein::SendMessageParams;
use frankenstein::{AsyncApi, UpdateContent};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use std::collections::HashMap;
use std::env;
use tokio::runtime;
use tokio_postgres::NoTls;
fn parse_string(bad: String) -> String {
    let mut parsed = String::new();
    for c in bad.chars() {
        match c {
            '.' => {
                parsed.push_str("\\.");
            }
            '!' => {
                parsed.push_str("\\!");
            }
            '-' => {
                parsed.push_str("\\-");
            }
            _ => {
                let mut b = [0; 4];
                parsed.push_str(c.encode_utf8(&mut b))
            }
        }
    }
    parsed
}
async fn send_message_client(
    chat_id: &i64,
    text: String,
    message: &Message,
    api: &AsyncApi,
) -> Result<(), Error> {
    let send_message_params = SendMessageParams::builder()
        .chat_id(*chat_id)
        .text(text)
        .reply_to_message_id(message.message_id)
        .parse_mode(ParseMode::MarkdownV2)
        .build();
    api.send_message(&send_message_params).await?;
    Ok(())
}
// En esta funcion el bot ya ha esperado a la respuesta del usuario
async fn weather_response(conf: Conf<'_>) -> Result<(), Error> {
    let v: Vec<&str> = conf.message.text.as_ref().unwrap().split(",").collect();
    let n = v.len();
    if n < 2 {
        let text = format!(
            "Hi, {}\\! Write it in the correct format please like this Madrid,ES or New York,US,NY",
            conf.username
        );
        send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
        return Ok(());
    }
    let city = v[0].trim();
    let country = v[1].trim();
    let mut state = "";
    if n == 3 {
        state = v[2].trim();
    }
    let (lon, lat, city_fmt, country_fmt, state_fmt) = match search_city(
        (*city).to_string(),
        (*country).to_string(),
        (*state).to_string(),
        conf.cities,
    )
    .await
    {
        Ok((lon, lat, city_fmt, country_fmt, state_fmt)) => {
            (lon, lat, city_fmt, country_fmt, state_fmt)
        }
        Err(_) => (
            -181.0,
            -91.0,
            String::from(""),
            String::from(""),
            String::from(""),
        ),
    };
    if lat == -91.0 {
        println!("User {} ,  City {} not found", conf.username, city);
        let text = format!(
            "Hi, {}\\! Your city {} was not found",
            conf.username,
            conf.message.text.as_ref().unwrap()
        );
        send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;

        return Ok(());
    }
    // spanish sp and english en
    // units metric or imperial
    let request_url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units={}&lang={}",
        lat, lon, conf.opwm_token, "metric", "sp",
    );
    let response = reqwest::get(&request_url).unwrap().text().unwrap();
    let weather_info = parse_weather(response).await.unwrap();
    println!(
        "User {} ,  City {} , Country {}\nLon {} , Lat {} {}",
        conf.username, city_fmt, country_fmt, lon, lat, weather_info
    );
    let text = match n {
        2 => parse_string(format!(
            "User {} ,  City {} , Country {}\nLon {} , Lat {}{}",
            conf.username, city_fmt, country_fmt, lon, lat, weather_info,
        )),
        3 => parse_string(format!(
            "User {} ,  City {} , State {} , Country {}\nLon {} , Lat {}{}",
            conf.username, city_fmt, state_fmt, country_fmt, lon, lat, weather_info,
        )),
        _ => panic!("wtf is this ?"),
    };
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
async fn city(conf: Conf<'_>) -> Result<(), Error> {
    let text = format!(
        "Hi, {}\\! Write city and country acronym like this Madrid,ES
        or for US states specify like this New York,US,NY being city,country,state",
        conf.username
    );
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
struct Conf<'a> {
    api: &'a AsyncApi,
    chat_id: &'a i64,
    username: &'a String,
    message: Message,
    cities: &'a HashMap<(String, String, String), (f64, f64, String, String, String)>,
    opwm_token: &'a String,
}
struct ProcessMessage {
    api: AsyncApi,
    message: Message,
    cities: HashMap<(String, String, String), (f64, f64, String, String, String)>,
    opwm_token: String,
    keypair: PKey<Private>,
}

fn main() {
    let rt = runtime::Builder::new_multi_thread()
        .worker_threads(6)
        .thread_name("my thread")
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    // Execution of code
    rt.block_on(async {
        bot_main().await.unwrap();
    });
}

async fn bot_main() -> Result<(), Error> {
    let token = env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    let api = AsyncApi::new(&token);
    let opwm_token =
        env::var("OPEN_WEATHER_MAP_API_TOKEN").expect("OPEN_WEATHER_MAP_API_TOKEN not set");
    let binary_file = std::fs::read("./resources/key.pem").unwrap();
    let keypair = Rsa::private_key_from_pem(&binary_file).unwrap();
    let keypair = PKey::from_rsa(keypair).unwrap();
    // Maybe cities should be in the database dude
    let json = tokio::spawn(async { read_json_cities() }).await.unwrap();
    // Fetch new updates via long poll method
    let update_params_builder = GetUpdatesParams::builder();
    let mut update_params = update_params_builder.clone().build();
    loop {
        let result = api.get_updates(&update_params).await;
        match result {
            Ok(response) => {
                for update in response.result {
                    if let UpdateContent::Message(message) = update.content {
                        let api_clone = api.clone();
                        let json_clone = json.clone();
                        let token_clone = opwm_token.clone();
                        let keypair_clone = keypair.clone();
                        let pm = ProcessMessage {
                            api: api_clone,
                            message: message,
                            cities: json_clone,
                            opwm_token: token_clone,
                            keypair: keypair_clone,
                        };
                        tokio::spawn(async move { process_message(pm).await.unwrap() });

                        update_params = update_params_builder
                            .clone()
                            .offset(update.update_id + 1)
                            .build();
                    }
                }
            }
            Err(error) => {
                println!("Failed to get updates: {:?}", error);
            }
        }
    }
}
async fn send_typing(message: &Message, api: &AsyncApi) -> Result<(), Error> {
    let send_chat_action_params = SendChatActionParams::builder()
        .chat_id((*((*message).chat)).id)
        .action(ChatAction::Typing)
        .build();
    api.send_chat_action(&send_chat_action_params).await?;
    Ok(())
}
async fn process_message(pm: ProcessMessage) -> Result<(), Error> {
    let chat_id: i64 = (*pm.message.chat).id;
    let user = match &pm.message.from.as_deref() {
        Some(user) => match &user.username {
            Some(username) => format!("@{}", (username).clone()),
            None => user.first_name.clone(),
        },
        None => panic!("No user ???"),
    };
    send_typing(&pm.message, &pm.api).await?;
    let (mut client, connection) =
        tokio_postgres::connect("host=localhost dbname=weather_bot user=postgres", NoTls)
            .await
            .unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    let mut transaction = client.transaction().await.unwrap();
    let state: String;
    if !is_in_db(&mut transaction, &chat_id).await.unwrap() {
        insert_client(
            &mut transaction,
            &chat_id,
            user.clone(),
            String::new(),
            &pm.keypair,
        )
        .await
        .unwrap();
        state = String::from("IN");
    } else {
        state = get_client_state(&mut transaction, &chat_id).await.unwrap();
    }
    let conf = Conf {
        api: &pm.api,
        chat_id: &chat_id,
        username: &user,
        message: pm.message.clone(),
        cities: &pm.cities,
        opwm_token: &pm.opwm_token,
    };
    match state.as_str() {
        "IN" => match pm.message.text.as_deref() {
            Some("/start") => {}
            Some("/city") => {
                city(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("AC"))
                    .await
                    .unwrap();
            }
            _ => {}
        },
        "AC" => match pm.message.text.as_deref() {
            Some(_) => {
                weather_response(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("IN"))
                    .await
                    .unwrap();
            }
            _ => {}
        },
        "DF" => {}
        _ => panic!("wtf is this state {} ?", state),
    }
    transaction.commit().await.unwrap();
    Ok(())
}
