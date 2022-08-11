use bb8_postgres::tokio_postgres::NoTls;
use frankenstein::api_params::{ChatAction, SendChatActionParams};
use frankenstein::AsyncApi;
use frankenstein::AsyncTelegramApi;
use frankenstein::GetUpdatesParams;
use frankenstein::Message;
use frankenstein::ParseMode;
use frankenstein::SendMessageParams;
use frankenstein::UpdateContent;
use openssl::pkey::PKey;
use openssl::pkey::Private;
use openssl::rsa::Rsa;
use std::fmt::Write;
use tokio::runtime;
use weather_bot_rust::db::BotDbError;
use weather_bot_rust::db::ClientState;
use weather_bot_rust::db::DbController;
use weather_bot_rust::json_parse::*;
use weather_bot_rust::BotError;
use weather_bot_rust::BINARY_FILE;
use weather_bot_rust::OPEN_WEATHER_MAP_API_TOKEN;
use weather_bot_rust::RUST_TELEGRAM_BOT_TOKEN;

// Function to send a message to a client.
async fn send_message_client(
    chat_id: &i64,
    text: String,
    message: &Message,
    api: &AsyncApi,
) -> Result<(), BotError> {
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
async fn cancel(conf: Conf<'_>) -> Result<(), BotError> {
    let text = format!("Hi, {}!\n Your operation was canceled", conf.username);
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    conf.db_controller
        .modify_before_state(conf.chat_id, conf.user_id, ClientState::Initial)
        .await?;

    conf.db_controller
        .modify_state(conf.chat_id, conf.user_id, ClientState::Initial)
        .await?;

    Ok(())
}
// Function to get daily weather info from Open Weather Map
async fn get_weather(
    conf: &Conf<'_>,
    city: &str,
    country: &str,
    state: &str,
    n: usize,
) -> Result<(), BotError> {
    let (lon, lat, city_fmt, country_fmt, state_fmt) = match conf
        .db_controller
        .search_city(
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
        send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;

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
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    Ok(())
}
async fn pattern_response(conf: Conf<'_>) -> Result<(), BotError> {
    let number: usize = conf
        .message
        .text
        .as_ref()
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let selected = conf
        .db_controller
        .get_client_selected(conf.chat_id, conf.user_id)
        .await?;
    let (name, country, state) = match conf.db_controller.get_city_row(&selected, number).await {
        Ok((n, c, s)) => (n, c, s),
        Err(_) => (String::new(), String::new(), String::new()),
    };
    let n: usize = match state.as_str() {
        "" => 2,
        _ => 3,
    };

    match conf
        .db_controller
        .get_client_before_state(conf.chat_id, conf.user_id)
        .await?
    {
        ClientState::Initial => {
            Ok(get_weather(&conf, name.as_str(), country.as_str(), state.as_str(), n).await?)
        }
        ClientState::SetCity => {
            let record = match n {
                2 => {
                    format!("{},{}", name, country)
                }
                3 => {
                    format!("{},{},{}", name, country, state)
                }
                _ => {
                    panic!("wtf is this number")
                }
            };
            conf.db_controller
                .modify_city(conf.chat_id, conf.user_id, record)
                .await?;

            city_updated_message(&conf).await?;
            Ok(())
        }
        _ => {
            panic!("wtf is this context")
        }
    }
}

// What we do if users write /pattern in Initial state.
async fn pattern_city(conf: &Conf<'_>) -> Result<(), BotError> {
    let text = format!(
        "Hi, {}! Write a city , let me see if i find it",
        conf.username
    );
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    Ok(())
}

async fn not_default_message(conf: &Conf<'_>) -> Result<(), BotError> {
    let text = format!("Hi, {}! Setting default city...", conf.username);
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    Ok(())
}
// What we do if users write a city in AskingPattern state.
async fn find_city(conf: Conf<'_>) -> Result<(), BotError> {
    let pattern = conf.message.text.as_ref().unwrap();

    let vec = conf.db_controller.get_city_by_pattern(pattern).await?;

    if vec.is_empty() || vec.len() > 30 {
        let text = format!(
            "Hi, {}! Your city {} was not found , try again",
            conf.username, pattern,
        );
        send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
        return Err(BotError::DbError(BotDbError::CityNotFoundError));
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
        if state.is_empty() {
            writeln!(&mut text, "{}. {},{}", i, name, country)?;
        } else {
            writeln!(&mut text, "{}. {},{},{}", i, name, country, state)?;
        }
        i += 1;
    }
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;

    Ok(())
}
// What we do if we are in AskingNumber state and is not a number
async fn not_number_message(conf: Conf<'_>) -> Result<(), BotError> {
    let text = format!(
        "Hi, {}! That's not a positive number in the range, try again",
        conf.username
    );
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    Ok(())
}
async fn city_updated_message(conf: &Conf<'_>) -> Result<(), BotError> {
    let text = format!("Hi, {}! Your default city was updated", conf.username);
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    Ok(())
}

async fn set_city(conf: Conf<'_>, chat_id: &i64, user_id: u64) -> Result<(), BotError> {
    pattern_city(&conf).await?;
    conf.db_controller
        .modify_state(chat_id, user_id, ClientState::Pattern)
        .await?;
    conf.db_controller
        .modify_before_state(chat_id, user_id, ClientState::SetCity)
        .await?;
    Ok(())
}
struct Conf<'a> {
    api: &'a AsyncApi,
    db_controller: DbController,
    chat_id: &'a i64,
    user_id: u64,
    username: &'a str,
    message: Message,
    opwm_token: &'a str,
}
struct ProcessMessage<'a> {
    _me: String,
    api: AsyncApi,
    message: Message,
    opwm_token: &'a str,
    keypair: PKey<Private>,
}

fn main() {
    pretty_env_logger::init();

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

async fn bot_main() -> Result<(), BotError> {
    // Initial setup to run the bot
    let api = AsyncApi::new(&RUST_TELEGRAM_BOT_TOKEN);
    let keypair = Rsa::private_key_from_pem(&BINARY_FILE).unwrap();
    let keypair = PKey::from_rsa(keypair).unwrap();
    let me = api.get_me().await?.result.username.unwrap();
    // Cities are in database ?
    // See if we have the cities in db
    let (mut client, connection) = bb8_postgres::tokio_postgres::connect(
        "host=localhost dbname=weather_bot user=postgres",
        NoTls,
    )
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
                        let token_clone = &OPEN_WEATHER_MAP_API_TOKEN;
                        let keypair_clone = keypair.clone();
                        let me_clone = me.clone();
                        // What we need to Process a Message.
                        let pm = ProcessMessage {
                            _me: me_clone,
                            api: api_clone,
                            message,
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
async fn send_typing(message: &Message, api: &AsyncApi) -> Result<(), BotError> {
    let send_chat_action_params = SendChatActionParams::builder()
        .chat_id((*((*message).chat)).id)
        .action(ChatAction::Typing)
        .build();
    api.send_chat_action(&send_chat_action_params).await?;
    Ok(())
}
// Process the message of each update
async fn process_message(pm: ProcessMessage<'_>) -> Result<(), BotError> {
    // get the user that is writing the message
    let chat_id: i64 = (*pm.message.chat).id;
    let user_id: u64 = match &pm.message.from.as_deref() {
        Some(user) => user.id,
        None => panic!("No user ???"),
    };
    let user = match &pm.message.from.as_deref() {
        Some(user) => match &user.username {
            Some(username) => format!("@{}", (username).clone()),
            None => user.first_name.clone(),
        },
        None => panic!("No user ???"),
    };
    send_typing(&pm.message, &pm.api).await?;
    // check if the user is in the database.
    let db_controller = DbController::new().await?;

    let state: ClientState;
    // if user is not in the database, insert the user with Initial state
    // if it is , check its state.
    if !db_controller.check_user_exists(&chat_id, user_id).await? {
        db_controller
            .insert_client(&chat_id, user_id, user.clone(), &pm.keypair)
            .await?;
        state = ClientState::Initial;
    } else {
        state = db_controller.get_client_state(&chat_id, user_id).await?;
    }

    // All what we need to handle the updates.
    let conf = Conf {
        api: &pm.api,
        db_controller: db_controller.clone(),
        chat_id: &chat_id,
        user_id,
        username: &user,
        message: pm.message.clone(),
        opwm_token: pm.opwm_token,
    };
    // State Machine that handles the user update.
    // Match the state and the message to know what to do.
    match state {
        ClientState::Initial => match pm.message.text.as_deref() {
            Some("/default") | Some("/default@RustWeather77Bot") => {
                match db_controller.get_client_city(&chat_id, user_id).await {
                    Ok(formated) => {
                        let v: Vec<&str> = formated.as_str().split(',').collect();
                        let n = v.len();
                        let city = v[0];
                        let country = v[1];
                        let mut state = "";
                        if n == 3 {
                            state = v[2];
                        }
                        get_weather(&conf, city, country, state, n).await?
                    }
                    Err(_) => {
                        not_default_message(&conf).await?;
                        set_city(conf, &chat_id, user_id).await?;
                    }
                }
            }
            Some("/pattern") | Some("/pattern@RustWeather77Bot") => {
                pattern_city(&conf).await?;
                db_controller
                    .modify_state(&chat_id, user_id, ClientState::Pattern)
                    .await?;
            }
            Some("/set_city") | Some("/set_city@RustWeather77Bot") => {
                set_city(conf, &chat_id, user_id).await?;
            }
            _ => {}
        },

        ClientState::Pattern => match pm.message.text.as_deref() {
            Some("/cancel") | Some("/cancel@RustWeather77Bot") => {
                cancel(conf).await?;
            }
            Some(text) => {
                if (find_city(conf).await).is_ok() {
                    db_controller
                        .modify_selected(&chat_id, user_id, text.to_string())
                        .await?;
                    db_controller
                        .modify_state(&chat_id, user_id, ClientState::Number)
                        .await?;
                }
            }
            _ => {}
        },
        ClientState::Number => match pm.message.text.as_deref() {
            Some("/cancel") | Some("/cancel@RustWeather77Bot") => {
                cancel(conf).await?;
            }
            Some(text) => match text.parse::<usize>() {
                Ok(_) => {
                    pattern_response(conf).await?;
                    db_controller
                        .modify_before_state(&chat_id, user_id, ClientState::Initial)
                        .await?;
                    db_controller
                        .modify_state(&chat_id, user_id, ClientState::Initial)
                        .await?;
                }
                Err(_) => {
                    not_number_message(conf).await?;
                }
            },
            _ => {}
        },

        _ => {}
    }
    Ok(())
}
