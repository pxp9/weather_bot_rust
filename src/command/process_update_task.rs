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

const BOT_NAME: &str = "RustWeather77Bot";
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
    api: ApiClient,
    repo: Repo,
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
                return Err(BotError::UpdateNotMessage("no text".to_string()));
            }

            let text = message.text.clone().unwrap();

            let repo = Repo::new().await?;
            let api = ApiClient::new();

            let chat_id: i64 = message.chat.id;
            let user = message.from.clone().expect("User not set");
            let chat = repo.find_or_create_chat(&chat_id, user.id).await?;
            let username = match user.username {
                Some(name) => name,
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
            Err(BotError::UpdateNotMessage("no message".to_string()))
        }
    }

    pub async fn process(&self) -> Result<(), BotError> {
        if Command::Cancel == self.command {
            return self.cancel().await;
        }

        match self.chat.state {
            ClientState::Initial => self.process_initial().await,
            ClientState::FindCity => self.process_find_city().await,
            ClientState::Number => self.process_number().await,
            _ => Ok(()),
        }
    }

    async fn process_initial(&self) -> Result<(), BotError> {
        match self.command {
            Command::FindCity => {
                self.find_city_message().await?;

                self.repo
                    .modify_state(&self.chat.id, self.chat.user_id, ClientState::FindCity)
                    .await?;
            }
            Command::Start => {
                self.start_message().await?;
            }
            Command::SetDefaultCity => {
                self.set_city().await?;
            }
            Command::Default => match self.chat.default_city_id {
                Some(id) => {
                    let city = self.repo.search_city_by_id(&id).await?;

                    self.get_weather(city).await?
                }
                None => {
                    self.not_default_message().await?;

                    self.set_city().await?;
                }
            },
            _ => (),
        }

        Ok(())
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
                self.pattern_response(number).await?;
                self.return_to_initial().await?;
            }
            Err(_) => {
                self.not_number_message().await?;
            }
        }

        Ok(())
    }

    async fn pattern_response(&self, number: usize) -> Result<(), BotError> {
        let city = self
            .repo
            .get_city_row(&self.chat.selected.clone().unwrap(), number)
            .await?;

        match self.chat.state {
            ClientState::Initial => self.get_weather(city).await,

            ClientState::SetCity => {
                self.repo
                    .modify_default_city(&self.chat.id, self.chat.user_id, &city.id)
                    .await?;

                self.city_updated_message().await
            }
            _ => Ok(()),
        }
    }

    async fn find_city(&self) -> Result<(), BotError> {
        let vec = self.repo.get_city_by_pattern(&self.text).await?;

        if vec.is_empty() || vec.len() > 30 {
            let text = format!("Your city {} was not found , try again", self.text);
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

        self.send_message(&text).await?;

        Ok(())
    }

    async fn cancel(&self) -> Result<(), BotError> {
        let text = "Your operation was canceled";
        self.send_message(text).await?;

        self.return_to_initial().await?;

        Ok(())
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
        self.find_city_message().await?;

        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::FindCity)
            .await?;

        self.repo
            .modify_before_state(&self.chat.id, self.chat.user_id, ClientState::SetCity)
            .await?;

        Ok(())
    }

    async fn not_number_message(&self) -> Result<(), BotError> {
        let text = "That's not a positive number in the range, try again";

        self.send_message(text).await
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
        /pattern Ask weather info from any city worldwide.\n
        /set_city Set your default city.\n
        /default Provides weather info from default city.\n
        /schedule Schedules the bot to run daily to provide weather info from default city.\n
        It would be really greatful if you take a look my GitHub, look how much work has this bot.\n
        If you like this bot consider giving me a star on GitHub or if you would like to self run it, fork the proyect please.\n
        <a href=\"https://github.com/pxp9/weather_bot_rust\">RustWeatherBot GitHub repo</a>";

        self.send_message(text).await
    }

    async fn get_weather(&self, city: City) -> Result<(), BotError> {
        let weather_client = WeatherApiClient::builder()
            .lat(city.coord.lat)
            .lon(city.coord.lon)
            .build();

        let weather_info = weather_client.fetch().await?;

        let text = format!(
            "{},{}\nLon {} , Lat {}\n{}",
            city.name, city.country, city.coord.lon, city.coord.lat, weather_info,
        );

        self.send_message(&text).await
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
        let processor = UpdateProcessor::create(self.update.clone()).await.unwrap();

        if let Err(error) = processor.process().await {
            log::error!(
                "Failed to process the update {:?} - {:?}",
                self.update,
                error
            );
        }

        Ok(())
    }

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }
}
