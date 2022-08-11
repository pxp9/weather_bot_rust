use crate::db::{ClientState, DbController};
use crate::{BotError, BINARY_FILE, OPEN_WEATHER_MAP_API_TOKEN, RUST_TELEGRAM_BOT_TOKEN};
use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::asynk::AsyncError as Error;
use fang::serde::{Deserialize, Serialize};
use fang::typetag;
use fang::AsyncRunnable;
use frankenstein::{
    AsyncApi, AsyncTelegramApi, ChatAction, Message, ParseMode, SendChatActionParams,
    SendMessageParams, Update, UpdateContent,
};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;

const TASK_TYPE: &str = "update";

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "fang::serde")]
pub struct ProcessUpdateTask {
    update: Update,
}

impl ProcessUpdateTask {
    pub fn new(update: Update) -> Self {
        Self { update }
    }

    pub async fn process(&self) -> Result<(), BotError> {
        log::info!("Received a message {:?}", self.update);

        let api = AsyncApi::new(&RUST_TELEGRAM_BOT_TOKEN);
        // for now with _ to pass warnings
        let _opwm_token = &OPEN_WEATHER_MAP_API_TOKEN;
        let keypair = Rsa::private_key_from_pem(&BINARY_FILE).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        if let UpdateContent::Message(message) = &self.update.content {
            let (chat_id, user_id, user) = Self::get_info_from_message(message);

            Self::send_typing(message, &api).await?;

            let state = Self::fetch_state(&chat_id, user_id, user.clone(), &keypair).await?;

            match state {
                ClientState::Initial => match message.text.as_deref() {
                    Some("/start") | Some("/start@RustWeather77Bot") => {
                        Self::start(message, &user, &api).await?;
                    }
                    _ => {}
                },

                ClientState::Pattern => match message.text.as_deref() {
                    Some("/pattern") | Some("/pattern@RustWeather77Bot") => {}
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn fetch_state(
        chat_id: &i64,
        user_id: u64,
        user: String,
        keypair: &PKey<Private>,
    ) -> Result<ClientState, BotError> {
        // Maybe here can be recycled pool from AsyncQueue from Fang for now this is fine
        let db_controller = DbController::new().await?;
        let state: ClientState = if !db_controller.check_user_exists(chat_id, user_id).await? {
            db_controller
                .insert_client(chat_id, user_id, user, keypair)
                .await?;

            ClientState::Initial
        } else {
            db_controller.get_client_state(chat_id, user_id).await?
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

    pub async fn start(message: &Message, username: &str, api: &AsyncApi) -> Result<(), BotError> {
        let text = format!(
        "Hi, {}!\nThis bot provides weather info around the globe.\nIn order to use it put the command:\n
        /pattern ask weather info from city without format\n
        /set_city set your default city\n
        /default provides weather info from default city\n
        It would be really greatful if you take a look my GitHub, look how much work has this bot, if you like this bot give me
        an star or if you would like to self run it, fork the proyect please.\n
        <a href=\"https://github.com/pxp9/weather_bot_rust\">RustWeatherBot </a>",
        username
            );
        Self::send_message(message, &text, api).await?;
        Ok(())
    }
    pub async fn send_message(
        message: &Message,
        text: &str,
        api: &AsyncApi,
    ) -> Result<(), BotError> {
        let send_message_params = SendMessageParams::builder()
            .chat_id(message.chat.id)
            .text(text)
            .reply_to_message_id(message.message_id)
            .parse_mode(ParseMode::Html)
            .build();

        api.send_message(&send_message_params).await?;

        Ok(())
    }

    // Function to make the bot Typing ...
    pub async fn send_typing(message: &Message, api: &AsyncApi) -> Result<(), BotError> {
        let send_chat_action_params = SendChatActionParams::builder()
            .chat_id(message.chat.id)
            .action(ChatAction::Typing)
            .build();
        api.send_chat_action(&send_chat_action_params).await?;
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

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }
}
