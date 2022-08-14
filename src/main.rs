use bb8_postgres::tokio_postgres::NoTls;
use frankenstein::api_params::ChatAction;
use frankenstein::api_params::SendChatActionParams;
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
use weather_bot_rust::db::ClientState;
use weather_bot_rust::db::Repo;
use weather_bot_rust::json_parse::*;
use weather_bot_rust::telegram::handler::Handler;
use weather_bot_rust::workers;
use weather_bot_rust::{
    BotError, BINARY_FILE, OPEN_WEATHER_MAP_API_TOKEN, RUST_TELEGRAM_BOT_TOKEN,
};

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
// Function to get daily weather info from Open Weather Map
async fn get_weather(
    conf: &Conf<'_>,
    city: &str,
    country: &str,
    state: &str,
    n: usize,
) -> Result<(), BotError> {
    let (lon, lat, city_fmt, country_fmt, state_fmt) =
        match conf.repo.search_city(city, country, state).await {
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

async fn not_default_message(conf: &Conf<'_>) -> Result<(), BotError> {
    let text = format!("Hi, {}! Setting default city...", conf.username);
    send_message_client(conf.chat_id, text, &conf.message, conf.api).await?;
    Ok(())
}
async fn set_city(conf: Conf<'_>, chat_id: &i64, user_id: u64) -> Result<(), BotError> {
    // call pattern_city here
    conf.repo
        .modify_state(chat_id, user_id, ClientState::FindCity)
        .await?;
    conf.repo
        .modify_before_state(chat_id, user_id, ClientState::SetCity)
        .await?;
    Ok(())
}
struct Conf<'a> {
    api: &'a AsyncApi,
    repo: Repo,
    chat_id: &'a i64,
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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    workers::start_workers().await;

    let mut handler = Handler::new().await;

    handler.start().await;
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
        .chat_id((*message).chat.id)
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
    let repo = Repo::new().await?;

    let state: ClientState;
    // if user is not in the database, insert the user with Initial state
    // if it is , check its state.
    if !repo.check_user_exists(&chat_id, user_id).await? {
        repo.insert_client(&chat_id, user_id, user.clone(), &pm.keypair)
            .await?;
        state = ClientState::Initial;
    } else {
        state = repo.get_client_state(&chat_id, user_id).await?;
    }

    // All what we need to handle the updates.
    let conf = Conf {
        api: &pm.api,
        repo: repo.clone(),
        chat_id: &chat_id,
        username: &user,
        message: pm.message.clone(),
        opwm_token: pm.opwm_token,
    };
    // State Machine that handles the user update.
    // Match the state and the message to know what to do.
    match state {
        ClientState::Initial => match pm.message.text.as_deref() {
            Some("/default") | Some("/default@RustWeather77Bot") => {
                match repo.get_client_city(&chat_id, user_id).await {
                    Ok(formated) => {
                        let v: Vec<&str> = formated.as_str().split(',').collect();
                        let n = v.len();
                        let city = v[0];
                        let country = v[1];
                        let mut state = "";
                        if n == 3 {
                            state = v[2];
                        }
                        // Not deleted yet, because i get warnings if i delete get_weather
                        get_weather(&conf, city, country, state, n).await?
                    }
                    Err(_) => {
                        not_default_message(&conf).await?;
                        set_city(conf, &chat_id, user_id).await?;
                    }
                }
            }
            Some("/set_city") | Some("/set_city@RustWeather77Bot") => {
                set_city(conf, &chat_id, user_id).await?;
            }
            _ => {}
        },

        ClientState::FindCity => {}
        ClientState::Number => {}

        _ => {}
    }
    Ok(())
}
