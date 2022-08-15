use super::client::ApiClient;
use crate::db::BotDbError;
use crate::db::ClientState;
use crate::db::Repo;
use crate::open_weather_map::client::WeatherApiClient;
use crate::open_weather_map::City;
use crate::BotError;
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
use std::fmt::Write;
use typed_builder::TypedBuilder;

const BOT_NAME: &str = "RustWeather77Bot";
const ERROR_MESSAGE_FOR_USER: &str = "Failed to fullfill the request";

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
        let handle = &format!("{}@{}", command, BOT_NAME);
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

            if Self::check_command("/cancel", &text) {
                Self::cancel(&params).await?;
                return Ok(());
            }

            match state {
                ClientState::Initial => {
                    if Self::check_command("/find_city", &text) {
                        log::info!("Find City command");
                        Self::find_city_message(&params).await?;

                        params
                            .repo
                            .modify_state(&chat_id, user_id, ClientState::FindCity)
                            .await?;
                    } else if Self::check_command("/default", &text) {
                        log::info!("Default command");
                        match params
                            .repo
                            .get_client_default_city_id(&chat_id, user_id)
                            .await
                        {
                            Ok(id) => {
                                let city = params.repo.search_city_by_id(&id).await?;
                                Self::get_weather(&params, city).await?
                            }
                            Err(_) => {
                                Self::not_default_message(&params).await?;
                                Self::set_city(&params).await?;
                            }
                        }
                    } else if Self::check_command("/set_default_city", &text) {
                        log::info!("Set Default City command");
                        Self::set_city(&params).await?;
                    } else if Self::check_command("/schedule", &text) {
                        log::info!("Schedule command");
                    } else if Self::check_command("/start", &text) {
                        log::info!("Start command");
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
            params
                .repo
                .insert_client(&params.chat_id, params.user_id)
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
        /pattern Ask weather info from any city worldwide.\n
        /set_city Set your default city.\n
        /default Provides weather info from default city.\n
        /schedule Schedules the bot to run daily to provide weather info from default city.\n
        It would be really greatful if you take a look my GitHub, look how much work has this bot.\n
        If you like this bot consider giving me a star on GitHub or if you would like to self run it, fork the proyect please.\n
        <a href=\"https://github.com/pxp9/weather_bot_rust\">RustWeatherBot GitHub repo</a>",
        params.username
            );
        Self::send_message(params, text).await?;
        Ok(())
    }
    // What we do if users write /pattern in Initial state.
    async fn find_city_message(params: &Params) -> Result<(), BotError> {
        let text = format!(
            "Hi, {}! Write a city , let me see if i find it",
            params.username
        );
        Self::send_message(params, text).await?;
        Ok(())
    }

    async fn set_city(params: &Params) -> Result<(), BotError> {
        Self::find_city_message(params).await?;

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

        let city = match params.repo.get_city_row(&selected, number).await {
            Ok(city) => city,
            Err(error) => {
                log::error!("failed to get city {:?}", error);
                return Self::error_message(params).await;
            }
        };

        match params
            .repo
            .get_client_before_state(&params.chat_id, params.user_id)
            .await?
        {
            ClientState::Initial => Ok(Self::get_weather(params, city).await?),

            ClientState::SetCity => {
                params
                    .repo
                    .modify_default_city(&params.chat_id, params.user_id, &city.id)
                    .await?;

                Self::city_updated_message(params).await?;
                Ok(())
            }
            _ => {
                panic!("wtf is this state")
            }
        }
    }

    async fn get_weather(params: &Params, city: City) -> Result<(), BotError> {
        let weather_client = WeatherApiClient::builder()
            .lat(city.coord.lat)
            .lon(city.coord.lon)
            .build();

        let weather_info = weather_client.fetch().await?;

        let text = format!(
            "Hi {},\n{},{}\nLon {} , Lat {}\n{}",
            params.username, city.name, city.country, city.coord.lon, city.coord.lat, weather_info,
        );
        Self::send_message(params, text).await?;
        Ok(())
    }

    async fn send_message(params: &Params, text: String) -> Result<(), BotError> {
        params.api.send_message(&params.message, text).await?;

        Ok(())
    }

    async fn error_message(params: &Params) -> Result<(), BotError> {
        Self::send_message(params, ERROR_MESSAGE_FOR_USER.to_string()).await?;

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
