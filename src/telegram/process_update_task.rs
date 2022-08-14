use super::client::ApiClient;
use crate::db::BotDbError;
use crate::db::ClientState;
use crate::db::Repo;
use crate::json_parse::*;
use crate::BotError;
use crate::BINARY_FILE;
use crate::OPEN_WEATHER_MAP_API_TOKEN;
use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::asynk::AsyncError as Error;
use fang::serde::Deserialize;
use fang::serde::Serialize;
use fang::typetag;
use fang::AsyncRunnable;
use frankenstein::Message;
use frankenstein::Update;
use frankenstein::UpdateContent;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use std::fmt::Write;
use typed_builder::TypedBuilder;

const BOT_NAME: &str = "RustWeather77Bot";

#[derive(TypedBuilder)]
pub struct Params {
    api: ApiClient,
    repo: Repo,
    chat_id: i64,
    user_id: u64,
    username: String,
    message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "fang::serde")]
pub struct ProcessUpdateTask {
    update: Update,
}

impl ProcessUpdateTask {
    pub fn new(update: Update) -> Self {
        Self { update }
    }

    fn get_text_from_message(message: &Message) -> Option<&str> {
        message.text.as_deref()
    }

    fn check_command(command: &str, text: &Option<&str>) -> bool {
        let handle = &format!("/{}@{}", command, BOT_NAME);
        *text == Some(command) || *text == Some(handle)
    }

    async fn return_to_initial(params: &Params) -> Result<(), BotError> {
        params
            .repo
            .modify_before_state(&params.chat_id, params.user_id, ClientState::Initial)
            .await?;
        params
            .repo
            .modify_state(&params.chat_id, params.user_id, ClientState::Initial)
            .await?;
        Ok(())
    }

    pub async fn process(&self) -> Result<(), BotError> {
        log::info!("Received a message {:?}", self.update);

        let api = ApiClient::new();
        let repo = Repo::new().await?;

        if let UpdateContent::Message(message) = &self.update.content {
            let (chat_id, user_id, username) = Self::get_info_from_message(message);

            let params = Params::builder()
                .api(api)
                .repo(repo)
                .chat_id(chat_id)
                .user_id(user_id)
                .username(username)
                .message(message.clone())
                .build();

            Self::send_typing(&params).await?;

            let state = Self::fetch_state(&params).await?;

            let text = Self::get_text_from_message(message);

            if Self::check_command("cancel", &text) {
                Self::cancel(&params).await?;
                return Ok(());
            }

            match state {
                ClientState::Initial => {
                    if Self::check_command("pattern", &text) {
                        Self::pattern_city(&params).await?;

                        params
                            .repo
                            .modify_state(&chat_id, user_id, ClientState::FindCity)
                            .await?;
                    } else if Self::check_command("default", &text) {
                        match params.repo.get_client_city(&chat_id, user_id).await {
                            Ok(formated) => {
                                let vec: Vec<&str> = formated.as_str().split(',').collect();
                                let n = vec.len();
                                let city = vec[0];
                                let country = vec[1];
                                let mut state = "";
                                if n == 3 {
                                    state = vec[2];
                                }
                                // Not deleted yet, because i get warnings if i delete get_weather
                                Self::get_weather(&params, city, country, state, n).await?
                            }
                            Err(_) => {
                                Self::not_default_message(&params).await?;
                                Self::set_city(&params).await?;
                            }
                        }
                    } else if Self::check_command("start", &text) {
                        Self::start(&params).await?;
                    }
                }

                ClientState::FindCity => {
                    if let Some(pattern) = &text {
                        if (Self::find_city(&params).await).is_ok() {
                            params
                                .repo
                                .modify_selected(&chat_id, user_id, pattern.to_string())
                                .await?;
                            params
                                .repo
                                .modify_state(&chat_id, user_id, ClientState::Number)
                                .await?;
                        }
                    }
                }
                ClientState::Number => {
                    if let Some(number) = &text {
                        match number.parse::<usize>() {
                            Ok(_) => {
                                Self::pattern_response(&params).await?;
                                Self::return_to_initial(&params).await?;
                            }
                            Err(_) => {
                                Self::not_number_message(&params).await?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn fetch_state(params: &Params) -> Result<ClientState, BotError> {
        // Maybe here can be recycled pool from AsyncQueue from Fang for now this is fine
        let state: ClientState = if !params
            .repo
            .check_user_exists(&params.chat_id, params.user_id)
            .await?
        {
            let keypair = Rsa::private_key_from_pem(&BINARY_FILE).unwrap();
            let keypair = PKey::from_rsa(keypair).unwrap();

            params
                .repo
                .insert_client(
                    &params.chat_id,
                    params.user_id,
                    params.username.to_string(),
                    &keypair,
                )
                .await?;

            ClientState::Initial
        } else {
            params
                .repo
                .get_client_state(&params.chat_id, params.user_id)
                .await?
        };

        Ok(state)
    }

    fn get_info_from_message(message: &Message) -> (i64, u64, String) {
        let chat_id: i64 = message.chat.id;
        let user_id: u64 = message.from.as_ref().expect("No user ???").id;

        let user = match &message.from.as_ref().expect("No user ???").username {
            Some(username) => format!("@{}", username.clone()),
            None => message
                .from
                .as_ref()
                .expect("No user ???")
                .first_name
                .clone(),
        };

        (chat_id, user_id, user)
    }

    async fn start(params: &Params) -> Result<(), BotError> {
        let text = format!(
        "Hi, {}!\nThis bot provides weather info around the globe.\nIn order to use it put the command:\n
        /pattern ask weather info from city without format\n
        /set_city set your default city\n
        /default provides weather info from default city\n
        It would be really greatful if you take a look my GitHub, look how much work has this bot, if you like this bot give me
        an star or if you would like to self run it, fork the proyect please.\n
        <a href=\"https://github.com/pxp9/weather_bot_rust\">RustWeatherBot </a>",
        params.username
            );
        Self::send_message(params, text).await?;
        Ok(())
    }
    // What we do if users write /pattern in Initial state.
    async fn pattern_city(params: &Params) -> Result<(), BotError> {
        let text = format!(
            "Hi, {}! Write a city , let me see if i find it",
            params.username
        );
        Self::send_message(params, text).await?;
        Ok(())
    }
    async fn set_city(params: &Params) -> Result<(), BotError> {
        // call pattern_city here
        params
            .repo
            .modify_state(&params.chat_id, params.user_id, ClientState::FindCity)
            .await?;
        params
            .repo
            .modify_before_state(&params.chat_id, params.user_id, ClientState::SetCity)
            .await?;
        Ok(())
    }
    // What we do if we are in AskingNumber state and is not a number
    async fn not_number_message(params: &Params) -> Result<(), BotError> {
        let text = format!(
            "Hi, {}! That's not a positive number in the range, try again",
            params.username
        );
        Self::send_message(params, text).await?;
        Ok(())
    }

    async fn not_default_message(params: &Params) -> Result<(), BotError> {
        let text = format!("Hi, {}! Setting default city...", params.username);
        Self::send_message(params, text).await?;
        Ok(())
    }

    async fn city_updated_message(params: &Params) -> Result<(), BotError> {
        let text = format!("Hi, {}! Your default city was updated", params.username);
        Self::send_message(params, text).await?;
        Ok(())
    }

    // What we do if users write /cancel in any state
    async fn cancel(params: &Params) -> Result<(), BotError> {
        let text = format!("Hi, {}!\n Your operation was canceled", params.username);
        Self::send_message(params, text).await?;

        Self::return_to_initial(params).await?;

        Ok(())
    }

    async fn find_city(params: &Params) -> Result<(), BotError> {
        let pattern = params.message.text.as_ref().unwrap();

        let vec = params.repo.get_city_by_pattern(pattern).await?;

        if vec.is_empty() || vec.len() > 30 {
            let text = format!(
                "Hi, {}! Your city {} was not found , try again",
                params.username, pattern,
            );
            Self::send_message(params, text).await?;
            return Err(BotError::DbError(BotDbError::CityNotFoundError));
        }

        let mut i = 1;
        let mut text: String = format!(
            "Hi {}, i found these cities put a number to select one\n\n",
            params.username
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
        Self::send_message(params, text).await?;

        Ok(())
    }

    async fn pattern_response(params: &Params) -> Result<(), BotError> {
        let number: usize = params
            .message
            .text
            .as_ref()
            .unwrap()
            .parse::<usize>()
            .unwrap();

        let selected = params
            .repo
            .get_client_selected(&params.chat_id, params.user_id)
            .await?;

        let (name, country, state) = match params.repo.get_city_row(&selected, number).await {
            Ok((n, c, s)) => (n, c, s),
            Err(_) => (String::new(), String::new(), String::new()),
        };
        let n: usize = match state.as_str() {
            "" => 2,
            _ => 3,
        };

        match params
            .repo
            .get_client_before_state(&params.chat_id, params.user_id)
            .await?
        {
            ClientState::Initial => {
                Ok(
                    Self::get_weather(params, name.as_str(), country.as_str(), state.as_str(), n)
                        .await?,
                )
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

                params
                    .repo
                    .modify_city(&params.chat_id, params.user_id, record)
                    .await?;

                Self::city_updated_message(params).await?;
                Ok(())
            }
            _ => {
                panic!("wtf is this state")
            }
        }
    }

    async fn get_weather(
        params: &Params,
        city: &str,
        country: &str,
        state: &str,
        n: usize,
    ) -> Result<(), BotError> {
        let (lon, lat, city_fmt, country_fmt, state_fmt) =
            match params.repo.search_city(city, country, state).await {
                Ok((lon, lat, city_fmt, country_fmt, state_fmt)) => {
                    (lon, lat, city_fmt, country_fmt, state_fmt)
                }
                Err(_) => {
                    println!(
                        "User {} ,  City {} not found",
                        params.username,
                        params.message.text.as_ref().unwrap()
                    );
                    let text = format!(
                        "Hi, {}! Your city {} was not found",
                        params.username,
                        params.message.text.as_ref().unwrap()
                    );
                    Self::send_message(params, text).await?;

                    return Ok(());
                }
            };

        // spanish sp and english en
        // units metric or imperial
        let opwm_token: &str = &OPEN_WEATHER_MAP_API_TOKEN;
        let request_url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units={}&lang={}",
        lat, lon, opwm_token, "metric", "sp",
        );

        let response = reqwest::get(&request_url).unwrap().text().unwrap();
        let weather_info = parse_weather(response).await.unwrap();
        println!(
            "User {} ,  City {} , Country {}\nLon {} , Lat {} {}",
            params.username, city_fmt, country_fmt, lon, lat, weather_info
        );
        let text = match n {
            2 => format!(
                "Hi {},\n{},{}\nLon {} , Lat {}\n{}",
                params.username, city_fmt, country_fmt, lon, lat, weather_info,
            ),
            3 => format!(
                "Hi {},\n{},{},{}\nLon {}  Lat {}\n{}",
                params.username, city_fmt, country_fmt, state_fmt, lon, lat, weather_info,
            ),
            _ => panic!("wtf is this ?"),
        };
        Self::send_message(params, text).await?;
        Ok(())
    }

    async fn send_message(params: &Params, text: String) -> Result<(), BotError> {
        params.api.send_message(&params.message, text).await?;

        Ok(())
    }

    // Function to make the bot Typing ...
    async fn send_typing(params: &Params) -> Result<(), BotError> {
        params.api.send_typing(&params.message).await?;

        Ok(())
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for ProcessUpdateTask {
    async fn run(&self, _queueable: &mut dyn AsyncQueueable) -> Result<(), Error> {
        self.process().await.unwrap();

        Ok(())
    }
}
