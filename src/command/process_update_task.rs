use crate::db::BotDbError;
use crate::db::Chat;
use crate::db::ClientState;
use crate::db::Repo;
use crate::open_weather_map::client::WeatherApiClient;
use crate::open_weather_map::City;
use crate::telegram::client::ApiClient;
use crate::BotError;
use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::asynk::AsyncError as Error;
use fang::serde::Deserialize;
use fang::serde::Serialize;
use fang::typetag;
use fang::AsyncRunnable;
use frankenstein::Update;
use frankenstein::UpdateContent;
use std::fmt::Write;
use std::str::FromStr;
use typed_builder::TypedBuilder;

const BOT_NAME: &str = "@RustWeather77Bot";
pub const TASK_TYPE: &str = "process_update";

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "fang::serde")]
pub struct ProcessUpdateTask {
    update: Update,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Command {
    Default,
    FindCity,
    SetDefaultCity,
    Start,
    Cancel,
    UnknownCommand(String),
}

#[derive(TypedBuilder)]
pub struct UpdateProcessor {
    api: &'static ApiClient,
    repo: &'static Repo,
    text: String,
    message_id: i32,
    username: String,
    command: Command,
    chat: Chat,
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command_str = s.replace(BOT_NAME, "");

        let result = match command_str.trim() {
            "/start" => Command::Start,
            "/find_city" => Command::FindCity,
            "/default" => Command::Default,
            "/set_default_city" => Command::SetDefaultCity,
            "/cancel" => Command::Cancel,
            _ => Command::UnknownCommand(command_str.to_string()),
        };

        Ok(result)
    }
}

impl UpdateProcessor {
    pub async fn create(update: Update) -> Result<Self, BotError> {
        if let UpdateContent::Message(message) = &update.content {
            if message.text.is_none() {
                log::error!("Update doesn't contain any text {:?}", message);

                return Err(BotError::UpdateNotMessage("no text".to_string()));
            }

            let text = message.text.clone().unwrap();

            let repo = Repo::repo().await?;
            let api = ApiClient::api_client().await;

            let chat_id: i64 = message.chat.id;
            let user = message.from.clone().expect("User not set");
            let chat = repo.find_or_create_chat(&chat_id, user.id).await?;
            let username = match user.username {
                Some(name) => format!("@{}", name),
                None => user.first_name,
            };

            let command = Command::from_str(&text).unwrap();

            let processor = Self::builder()
                .repo(repo)
                .api(api)
                .message_id(message.message_id)
                .text(text)
                .username(username)
                .chat(chat)
                .command(command)
                .build();

            Ok(processor)
        } else {
            log::error!("Update is not a message {:?}", update);

            Err(BotError::UpdateNotMessage("no message".to_string()))
        }
    }

    pub async fn process(&self) -> Result<(), BotError> {
        self.send_typing().await?;

        if Command::Cancel == self.command {
            return self.cancel(None).await;
        }

        match self.chat.state {
            ClientState::Initial => self.process_initial().await,
            ClientState::FindCity => self.process_find_city().await,
            ClientState::Number => self.process_number().await,
            _ => self.revert_state().await,
        }
    }

    async fn process_initial(&self) -> Result<(), BotError> {
        match self.command {
            Command::FindCity => {
                self.repo
                    .modify_state(&self.chat.id, self.chat.user_id, ClientState::FindCity)
                    .await?;

                self.find_city_message().await?;

                Ok(())
            }
            Command::Start => self.start_message().await,
            Command::SetDefaultCity => self.set_city().await,
            Command::Default => match self.chat.default_city_id {
                Some(id) => {
                    let city = self.repo.search_city_by_id(&id).await?;

                    self.get_weather(city).await
                }
                None => {
                    self.set_city().await?;

                    self.not_default_message().await
                }
            },
            _ => self.unknown_command().await,
        }
    }

    async fn process_find_city(&self) -> Result<(), BotError> {
        self.find_city().await?;

        self.repo
            .modify_selected(&self.chat.id, self.chat.user_id, self.text.clone())
            .await?;

        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::Number)
            .await?;

        Ok(())
    }

    async fn process_number(&self) -> Result<(), BotError> {
        match self.text.parse::<usize>() {
            Ok(number) => {
                self.return_to_initial().await?;

                self.pattern_response(number).await
            }

            Err(_) => self.not_number_message().await,
        }
    }

    async fn pattern_response(&self, number: usize) -> Result<(), BotError> {
        let city = self
            .repo
            .get_city_row(&self.chat.selected.clone().unwrap(), number)
            .await?;

        match self.chat.before_state {
            ClientState::Initial => self.get_weather(city).await,

            ClientState::SetCity => self.set_default_city(city).await,

            _ => self.revert_state().await,
        }
    }

    async fn find_city(&self) -> Result<(), BotError> {
        let vec = self.repo.get_city_by_pattern(&self.text).await?;

        if vec.is_empty() || vec.len() > 30 {
            let text = format!("Your city {} was not found", self.text);
            self.send_message(&text).await?;

            return Err(BotError::DbError(BotDbError::CityNotFoundError));
        }

        let mut i = 1;
        let mut text: String = "I found these cities. Put a number to select one\n\n".to_string();

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

        self.send_message(&text).await
    }

    async fn cancel(&self, custom_message: Option<String>) -> Result<(), BotError> {
        self.return_to_initial().await?;

        let text = match custom_message {
            Some(message) => message,
            None => "Your operation was canceled".to_string(),
        };
        self.send_message(&text).await
    }

    async fn revert_state(&self) -> Result<(), BotError> {
        self.cancel(Some(
            "Failed to process your command. Please try to run the command again".to_string(),
        ))
        .await
    }

    async fn unknown_command(&self) -> Result<(), BotError> {
        self.cancel(Some(
            "Unknown command. See /start for available commands".to_string(),
        ))
        .await
    }

    async fn return_to_initial(&self) -> Result<(), BotError> {
        self.repo
            .modify_before_state(&self.chat.id, self.chat.user_id, ClientState::Initial)
            .await?;

        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::Initial)
            .await?;

        Ok(())
    }

    async fn set_city(&self) -> Result<(), BotError> {
        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::FindCity)
            .await?;

        self.repo
            .modify_before_state(&self.chat.id, self.chat.user_id, ClientState::SetCity)
            .await?;

        self.find_city_message().await
    }

    async fn not_number_message(&self) -> Result<(), BotError> {
        self.cancel(Some(
            "That's not a positive number in the range. The command was cancelled".to_string(),
        ))
        .await
    }

    async fn city_updated_message(&self) -> Result<(), BotError> {
        let text = "Your default city was updated";

        self.send_message(text).await
    }

    async fn find_city_message(&self) -> Result<(), BotError> {
        let text = "Write a city, let me see if I can find it";

        self.send_message(text).await
    }

    async fn start_message(&self) -> Result<(), BotError> {
        let text = "This bot provides weather info around the globe.\nIn order to use it put the command:\n
        /find_city Ask weather info from any city worldwide.\n
        /set_default_city Set your default city.\n
        /default Provides weather info from default city.\n
        It would be really greatful if you take a look at my GitHub, look how much work I invested into this bot.\n
        If you like this bot, consider giving me a star on GitHub or if you would like to self run it, fork the project please.\n
        <a href=\"https://github.com/pxp9/weather_bot_rust\">RustWeatherBot GitHub repo</a>";

        self.send_message(text).await
    }

    async fn get_weather(&self, city: City) -> Result<(), BotError> {
        let weather_client = WeatherApiClient::weather_client().await;

        let weather_info = weather_client.fetch(city.coord.lat, city.coord.lon).await?;

        let text = format!(
            "{},{}\nLat {} , Lon {}\n{}",
            city.name, city.country, city.coord.lat, city.coord.lon, weather_info,
        );

        self.send_message(&text).await
    }

    async fn set_default_city(&self, city: City) -> Result<(), BotError> {
        self.repo
            .modify_default_city(&self.chat.id, self.chat.user_id, &city.id)
            .await?;

        self.city_updated_message().await
    }

    async fn not_default_message(&self) -> Result<(), BotError> {
        let text = "Setting default city...";

        self.send_message(text).await
    }

    async fn send_message(&self, text: &str) -> Result<(), BotError> {
        let text_with_username = format!("Hi, {}!\n{}", self.username, text);

        self.api
            .send_message(self.chat.id, self.message_id, text_with_username)
            .await?;

        Ok(())
    }
    async fn send_typing(&self) -> Result<(), BotError> {
        self.api.send_typing(self.chat.id).await?;
        Ok(())
    }
}

impl ProcessUpdateTask {
    pub fn new(update: Update) -> Self {
        Self { update }
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for ProcessUpdateTask {
    async fn run(&self, _queueable: &mut dyn AsyncQueueable) -> Result<(), Error> {
        let processor = match UpdateProcessor::create(self.update.clone()).await {
            Ok(processor) => processor,
            Err(err) => {
                log::error!("Failed to initialize the processor {:?}", err);

                return Ok(());
            }
        };

        if let Err(error) = processor.process().await {
            log::error!(
                "Failed to process the update {:?} - {:?}. Reverting...",
                self.update,
                error
            );

            if let Err(err) = processor.revert_state().await {
                log::error!("Failed to revert: {:?}", err);
            }
        }

        Ok(())
    }

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }
}
