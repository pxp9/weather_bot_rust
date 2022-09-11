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
use fang::serde::Deserialize;
use fang::serde::Serialize;
use fang::typetag;
use fang::AsyncRunnable;
use fang::FangError;
use fang::Scheduled;
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
    Schedule,
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
            "/schedule" => Command::Schedule,
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

    pub async fn process(&self) -> Result<Option<ScheduleWeatherTask>, BotError> {
        self.send_typing().await?;

        if Command::Cancel == self.command {
            self.cancel(None).await?;
            return Ok(None);
        }

        match self.chat.state {
            ClientState::Initial => {
                self.process_initial().await?;
                Ok(None)
            }
            ClientState::FindCity => {
                self.process_find_city().await?;
                Ok(None)
            }
            ClientState::SetCity => {
                self.process_set_city().await?;
                Ok(None)
            }
            ClientState::Time => {
                self.process_time().await?;
                Ok(None)
            }
            ClientState::FindCityNumber => {
                self.process_find_city_number().await?;
                Ok(None)
            }
            ClientState::SetCityNumber => {
                self.process_set_city_number().await?;
                Ok(None)
            }
            ClientState::Offset => self.process_offset().await,
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
            Command::Schedule => self.schedule_weather().await,
            _ => self.unknown_command().await,
        }
    }

    async fn process_find_city(&self) -> Result<(), BotError> {
        self.find_city().await?;

        self.repo
            .modify_selected(&self.chat.id, self.chat.user_id, self.text.clone())
            .await?;

        self.repo
            .modify_state(
                &self.chat.id,
                self.chat.user_id,
                ClientState::FindCityNumber,
            )
            .await?;

        Ok(())
    }

    async fn process_set_city(&self) -> Result<(), BotError> {
        self.find_city().await?;

        self.repo
            .modify_selected(&self.chat.id, self.chat.user_id, self.text.clone())
            .await?;

        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::SetCityNumber)
            .await?;

        Ok(())
    }

    async fn process_find_city_number(&self) -> Result<(), BotError> {
        match self.text.parse::<usize>() {
            Ok(number) => {
                let city = self
                    .repo
                    .get_city_row(&self.chat.selected.clone().unwrap(), number)
                    .await?;

                self.return_to_initial().await?;

                self.get_weather(city).await
            }

            Err(_) => self.not_number_message().await,
        }
    }

    async fn process_set_city_number(&self) -> Result<(), BotError> {
        match self.text.parse::<usize>() {
            Ok(number) => {
                let city = self
                    .repo
                    .get_city_row(&self.chat.selected.clone().unwrap(), number)
                    .await?;

                self.return_to_initial().await?;

                self.set_default_city(city).await
            }

            Err(_) => self.not_number_message().await,
        }
    }

    async fn not_valid_offset_message(&self) -> Result<Option<ScheduleWeatherTask>, BotError> {
        self.cancel(Some(
            "That's not a valid offset, it has to be a number in range [-11, 12].\n
            If your timezone is UTC + 2 put 2, if you have UTC - 10 put -10, 0 if you have UTC timezone.\n
            The command was cancelled"
            .to_string(),
        ))
        .await?;

        Ok(None)
    }

    async fn process_offset(&self) -> Result<Option<ScheduleWeatherTask>, BotError> {
        match self.text.parse::<i8>() {
            Ok(number) => {
                if !(-11..=12).contains(&number) {
                    return self.not_valid_offset_message().await;
                }

                let offset = number;
                // we have hour and minutes well formatted stored in selected see in process_time func.
                let vec: Vec<&str> = self.chat.selected.as_ref().unwrap().split(':').collect();
                // This unwraps are safe because we know that is stored in the correct format.
                let user_hour = vec[0].parse::<i8>().unwrap();
                let minutes = vec[1].parse::<i8>().unwrap();

                let schedule_weather_task = ScheduleWeatherTask::builder()
                    .user_hour(user_hour)
                    .minutes(minutes)
                    .offset(offset)
                    .username(self.username.clone())
                    .chat_id(self.chat.id)
                    // Safe unwrap checked in process_time func.
                    .default_city_id(self.chat.default_city_id.unwrap())
                    .build();

                self.return_to_initial().await?;

                let minutes_pretty = if minutes < 10 {
                    format!("0{}", vec[1])
                } else {
                    vec[1].to_string()
                };

                let text = format!(
                    "Weather info scheduled every day at {}:{} UTC {}",
                    vec[0], minutes_pretty, offset
                );

                self.send_message(&text).await?;

                Ok(Some(schedule_weather_task))
            }

            Err(_) => self.not_valid_offset_message().await,
        }
    }

    async fn not_time_message(&self) -> Result<(), BotError> {
        self.cancel(Some(
            "That's not a well formatted time, it has to be formatted with this format `hour:minutes` being hour a number in range [0,23] 
            and minutes a number in range [0,59]. The command was cancelled"
            .to_string(),
        ))
        .await
    }

    fn parse_time(hour_or_minutes: &str, max_range: i32, min_range: i32) -> i32 {
        match hour_or_minutes.parse::<i32>() {
            Ok(number) => {
                if !(min_range..=max_range).contains(&number) {
                    -1
                } else {
                    number
                }
            }
            Err(_) => -1,
        }
    }

    async fn process_time(&self) -> Result<(), BotError> {
        // check if user has default city.
        if self.chat.default_city_id.is_none() {
            return self
                .cancel(Some(
                    "To use /schedule command default city must be set first.".to_string(),
                ))
                .await;
        }

        let vec: Vec<&str> = self.text.trim().split(':').collect();

        if vec.len() != 2 {
            return self.not_time_message().await;
        }

        let hour = match Self::parse_time(vec[0], 23, 0) {
            -1 => return self.not_time_message().await,
            number => number,
        };

        let minutes = match Self::parse_time(vec[1], 59, 0) {
            -1 => return self.not_time_message().await,
            number => number,
        };

        self.repo
            .modify_selected(
                &self.chat.id,
                self.chat.user_id,
                format!("{}:{}", hour, minutes),
            )
            .await?;

        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::Offset)
            .await?;

        let text = "Do you have any offset respect UTC ?\n 
            (0 if your timezone is the same as UTC, 2 if UTC + 2 , -2 if UTC - 2, [-11,12])";

        self.send_message(text).await?;

        Ok(())
    }

    async fn find_city(&self) -> Result<(), BotError> {
        let vec = self.repo.get_city_by_pattern(&self.text).await?;

        if vec.is_empty() || vec.len() > 30 {
            let text = format!("Your city {} was not found. Command cancelled.", self.text);
            self.send_message(&text).await?;

            // User state will get reverted after return this error.
            // Also will prompt an Error log in server. I will consider here,
            // delete this error and just call cancel func.
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
        self.cancel(None).await
    }

    async fn unknown_command(&self) -> Result<(), BotError> {
        self.cancel(Some(
            "Unknown command. See /start for available commands".to_string(),
        ))
        .await
    }

    async fn return_to_initial(&self) -> Result<(), BotError> {
        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::Initial)
            .await?;

        Ok(())
    }

    async fn schedule_weather_message(&self) -> Result<(), BotError> {
        let text =
            "what time would you like to schedule ? (format hour:minutes in range 0-23:0-59)";

        self.send_message(text).await
    }

    async fn schedule_weather(&self) -> Result<(), BotError> {
        //asking time
        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::Time)
            .await?;

        self.schedule_weather_message().await
    }

    async fn set_city(&self) -> Result<(), BotError> {
        self.repo
            .modify_state(&self.chat.id, self.chat.user_id, ClientState::SetCity)
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

#[derive(Serialize, Deserialize, Debug, TypedBuilder, Eq, PartialEq, Clone)]
#[serde(crate = "fang::serde")]
pub struct ScheduleWeatherTask {
    user_hour: i8,
    minutes: i8,
    offset: i8,
    username: String,
    chat_id: i64,
    default_city_id: i32,
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for ScheduleWeatherTask {
    async fn run(&self, _queueable: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        // here we should program the weather_info deliver
        let repo = Repo::repo().await?;
        let api = ApiClient::api_client().await;

        let city = repo.search_city_by_id(&self.default_city_id).await?;

        let weather_client = WeatherApiClient::weather_client().await;

        let weather_info = weather_client
            .fetch_weekly(city.coord.lat, city.coord.lon)
            .await
            .unwrap();

        let text = format!(
            "Hi {} !, this is your scheduled weather info.\n\n {},{}\nLat {} , Lon {}\n{}",
            self.username, city.name, city.country, city.coord.lat, city.coord.lon, weather_info,
        );

        api.send_message_without_reply(self.chat_id, text)
            .await
            .unwrap();

        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }

    fn cron(&self) -> Option<Scheduled> {
        let hour_utc = if self.user_hour - self.offset < 0 {
            self.user_hour - self.offset + 24
        } else {
            self.user_hour - self.offset
        };

        Some(Scheduled::CronPattern(format!(
            "0 {} {} * * * *",
            self.minutes, hour_utc
        )))
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
    async fn run(&self, queueable: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        let processor = match UpdateProcessor::create(self.update.clone()).await {
            Ok(processor) => processor,
            Err(err) => {
                log::error!("Failed to initialize the processor {:?}", err);

                return Ok(());
            }
        };

        match processor.process().await {
            Err(error) => {
                log::error!(
                    "Failed to process the update {:?} - {:?}. Reverting...",
                    self.update,
                    error
                );

                let result = processor.revert_state().await;

                if let Err(err) = result {
                    log::error!("Failed to revert: {:?}", err);
                }
            }
            Ok(Some(schedule_task)) => {
                queueable.schedule_task(&schedule_task).await?;
            }
            _ => {}
        };

        Ok(())
    }

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }
}
