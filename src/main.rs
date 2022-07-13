mod json_parse;
use crate::json_parse::*;
mod database_manage;
use crate::database_manage::*;
mod tests;
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
use std::env;
use tokio::runtime;
use tokio_postgres::NoTls;
// What we do if users write /start in any state.
async fn start(conf: Conf<'_>) -> Result<(), Error> {
    let text = format!(
        "Hi, {}!\nThis bot provides weather info around the globe.\nIn order to use it put the command:\n/city\n
The bot is going to ask a city in a specific format, finally the bot will provide the weather info.\n
It would be really greatful if you take a look my GitHub, look how much work has this bot, if you like this bot give me
an star or if you would like to self run it, fork the proyect please.\n
<a href=\"https://github.com/pxp9/weather_bot_rust\">RustWeatherBot </a>",
        conf.username
    );
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
// Function to send a message to a client.
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
        .parse_mode(ParseMode::Html)
        .build();
    api.send_message(&send_message_params).await?;
    Ok(())
}
// What we do if users write /cancel in any state
async fn cancel_message(conf: Conf<'_>) -> Result<(), Error> {
    let text = format!("Hi, {}!\n Your operation was canceled", conf.username);
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
// Function to get daily weather info from Open Weather Map
async fn get_weather(
    conf: &Conf<'_>,
    city: &str,
    country: &str,
    state: &str,
    n: usize,
) -> Result<(), Error> {
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
    let (lon, lat, city_fmt, country_fmt, state_fmt) = match search_city(
        &mut transaction,
        &(*city).to_string(),
        &(*country).to_string(),
        &(*state).to_string(),
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
        println!(
            "User {} ,  City {} not found",
            conf.username,
            conf.message.text.as_ref().unwrap()
        );
        let text = format!(
            "Hi, {}! Your city {} was not found",
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
        2 => format!(
            "Hi {},\n{},{}\nLon {} , Lat {}\n{}",
            conf.username, city_fmt, country_fmt, lon, lat, weather_info,
        ),
        3 => format!(
            "Hi {},\n{},{},{}\nLon {}  Lat {}\n{}",
            conf.username, city_fmt, country_fmt, state_fmt, lon, lat, weather_info,
        ),
        _ => panic!("wtf is this ?"),
    };
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
// What we do if users write a city in correct format that is in the DB in AskingCity state.
async fn city_response(conf: Conf<'_>) -> Result<(), Error> {
    let v: Vec<&str> = conf.message.text.as_ref().unwrap().split(",").collect();
    let n = v.len();
    if n < 2 {
        let text = format!(
            "Hi, {}! Write it in the correct format please like this:\n Madrid,ES or New York,US,NY",
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
    Ok(get_weather(&conf, city, country, state, n).await?)
}
// What we do if users write a number in AskingNumber state.
async fn pattern_response(conf: Conf<'_>) -> Result<(), Error> {
    let number: usize = conf
        .message
        .text
        .as_ref()
        .unwrap()
        .parse::<usize>()
        .unwrap();
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
    let selected = get_client_selected(&mut transaction, conf.chat_id)
        .await
        .unwrap();
    let (name, country, state) = match get_city_row(&mut transaction, &selected, number).await {
        Ok((n, c, s)) => (n, c, s),
        Err(_) => (String::new(), String::new(), String::new()),
    };
    let n: usize = match state.as_str() {
        "" => 2,
        _ => 3,
    };
    if name == "" {
        let text = format!("Hi, {}! That number is not in the range", conf.username);
        send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
        Ok(())
    } else {
        Ok(get_weather(&conf, name.as_str(), country.as_str(), state.as_str(), n).await?)
    }
}
// What we do if users write /city in Initial state.
async fn city(conf: Conf<'_>) -> Result<(), Error> {
    let text = format!(
        "Hi, {}! Write city and country acronym like this:\nMadrid,ES\nor for US states specify like this:\nNew York,US,NY being city,country,state",
        conf.username
    );
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
// What we do if users write /pattern in Initial state.
async fn pattern_city(conf: Conf<'_>) -> Result<(), Error> {
    let text = format!(
        "Hi, {}! Write a city , let me see if i find it",
        conf.username
    );
    send_message_client(conf.chat_id, text, &conf.message, &conf.api).await?;
    Ok(())
}
// What we do if users write a city in AskingPattern state.
async fn find_city(conf: Conf<'_>) -> Result<(), ()> {
    let pattern = conf.message.text.as_ref().unwrap();
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
    let vec = get_city_by_pattern(&mut transaction, &pattern)
        .await
        .unwrap();
    if vec.len() == 0 || vec.len() > 30 {
        let text = format!(
            "Hi, {}! Your city {} was not found , try again",
            conf.username, pattern,
        );
        send_message_client(conf.chat_id, text, &conf.message, &conf.api)
            .await
            .unwrap();
        return Err(());
    }
    let mut i = 1;
    let mut text: String = format!(
        "Hi {}, i found these cities put a number to select one\n\n",
        conf.username
    );
    for row in vec {
        let name: String = row.get("name");
        let country: String = row.get("country");
        let state: String = row.get("state");
        if state == "" {
            text += &format!("{}. {},{}\n", i, name, country);
        } else {
            text += &format!("{}. {},{},{}\n", i, name, country, state);
        }
        i += 1;
    }
    send_message_client(conf.chat_id, text, &conf.message, &conf.api)
        .await
        .unwrap();
    Ok(())
}
// What we do if we are in AskingNumber state and is not a number
async fn not_number_message(conf: Conf<'_>) -> Result<(), Error> {
    let text = format!(
        "Hi, {}! That's not a positive number in the range, try again",
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
    opwm_token: &'a String,
}
struct ProcessMessage {
    api: AsyncApi,
    message: Message,
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
    // Initial setup to run the bot
    let token = env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    let api = AsyncApi::new(&token);
    let opwm_token =
        env::var("OPEN_WEATHER_MAP_API_TOKEN").expect("OPEN_WEATHER_MAP_API_TOKEN not set");
    let binary_file = std::fs::read("./resources/key.pem").unwrap();
    let keypair = Rsa::private_key_from_pem(&binary_file).unwrap();
    let keypair = PKey::from_rsa(keypair).unwrap();
    // Cities are in database ?
    // See if we have the cities in db
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
    let n = transaction
        .execute("SELECT * FROM cities", &[])
        .await
        .unwrap();
    if n == 0 {
        read_json_cities(&mut transaction).await.unwrap();
        transaction.commit().await.unwrap();
    }
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
                        let token_clone = opwm_token.clone();
                        let keypair_clone = keypair.clone();
                        // What we need to Process a Message.
                        let pm = ProcessMessage {
                            api: api_clone,
                            message: message,
                            opwm_token: token_clone,
                            keypair: keypair_clone,
                        };
                        // For each update we process the message that has.
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
// Function to make the bot Typing ...
async fn send_typing(message: &Message, api: &AsyncApi) -> Result<(), Error> {
    let send_chat_action_params = SendChatActionParams::builder()
        .chat_id((*((*message).chat)).id)
        .action(ChatAction::Typing)
        .build();
    api.send_chat_action(&send_chat_action_params).await?;
    Ok(())
}
// Process the message of each update
async fn process_message(pm: ProcessMessage) -> Result<(), Error> {
    // get the user that is writing the message
    let chat_id: i64 = (*pm.message.chat).id;
    let user = match &pm.message.from.as_deref() {
        Some(user) => match &user.username {
            Some(username) => format!("@{}", (username).clone()),
            None => user.first_name.clone(),
        },
        None => panic!("No user ???"),
    };
    send_typing(&pm.message, &pm.api).await?;
    // check if the user is in the database.
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
    // if user is not in the database, insert the user with Initial state
    // if it is , check its state.
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
        state = String::from("Initial");
    } else {
        state = get_client_state(&mut transaction, &chat_id).await.unwrap();
    }
    // All what we need to handle the updates.
    let conf = Conf {
        api: &pm.api,
        chat_id: &chat_id,
        username: &user,
        message: pm.message.clone(),
        opwm_token: &pm.opwm_token,
    };
    // State Machine that handles the user update.
    // Match the state and the message to know what to do.
    match state.as_str() {
        "Initial" => match pm.message.text.as_deref() {
            Some("/start") => {
                start(conf).await?;
            }
            Some("/city") => {
                city(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("AskingCity"))
                    .await
                    .unwrap();
            }
            Some("/pattern") => {
                pattern_city(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("AskingPattern"))
                    .await
                    .unwrap();
            }
            _ => {}
        },
        "AskingCity" => match pm.message.text.as_deref() {
            Some("/start") => {
                start(conf).await?;
            }
            Some("/cancel") => {
                cancel_message(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("Initial"))
                    .await
                    .unwrap();
            }
            Some("/city") => {
                city(conf).await?;
            }
            Some(_) => {
                city_response(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("Initial"))
                    .await
                    .unwrap();
            }
            _ => {}
        },
        "AskingPattern" => match pm.message.text.as_deref() {
            Some("/start") => {
                start(conf).await?;
            }
            Some("/cancel") => {
                cancel_message(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("Initial"))
                    .await
                    .unwrap();
            }
            Some(text) => match find_city(conf).await {
                Ok(_) => {
                    modify_selected(&mut transaction, &chat_id, text.to_string())
                        .await
                        .unwrap();
                    modify_state(&mut transaction, &chat_id, String::from("AskingNumber"))
                        .await
                        .unwrap();
                }
                Err(()) => {}
            },
            _ => {}
        },
        "AskingNumber" => match pm.message.text.as_deref() {
            Some("/start") => {
                start(conf).await?;
            }
            Some("/cancel") => {
                cancel_message(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("Initial"))
                    .await
                    .unwrap();
            }
            Some(text) => match text.parse::<usize>() {
                Ok(_) => {
                    pattern_response(conf).await?;
                    modify_state(&mut transaction, &chat_id, String::from("Initial"))
                        .await
                        .unwrap();
                }
                Err(_) => {
                    not_number_message(conf).await?;
                }
            },
            _ => {}
        },
        "DefaultCity" => match pm.message.text.as_deref() {
            Some("/start") => {
                start(conf).await?;
            }
            Some("/cancel") => {
                cancel_message(conf).await?;
                modify_state(&mut transaction, &chat_id, String::from("Initial"))
                    .await
                    .unwrap();
            }
            _ => {}
        },
        _ => panic!("wtf is this state {} ?", state),
    }
    // VERY IMPORTANT
    // this will modify state of the user in the database and make it persist.
    // if we do not modify it, program should have serious issues.
    transaction.commit().await.unwrap();
    Ok(())
}
