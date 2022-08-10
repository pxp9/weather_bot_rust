use crate::BotError;
use crate::{BINARY_FILE, OPEN_WEATHER_MAP_API_TOKEN, RUST_TELEGRAM_BOT_TOKEN};
use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::asynk::AsyncError as Error;
use fang::serde::{Deserialize, Serialize};
use fang::typetag;
use fang::AsyncRunnable;
use frankenstein::{
    AsyncApi, AsyncTelegramApi, ChatAction, Message, ParseMode, SendChatActionParams,
    SendMessageParams, Update,
};
use openssl::pkey::PKey;
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

    pub async fn process(&self) {
        // TODO: process users messages here
        // Create a ApiClient
        let api = AsyncApi::new(&RUST_TELEGRAM_BOT_TOKEN);
        let opwm_token = &OPEN_WEATHER_MAP_API_TOKEN;
        let keypair = Rsa::private_key_from_pem(&BINARY_FILE).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        log::info!("Received a message {:?}", self.update);
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
    pub async fn send_typing(&self, message: &Message, api: &AsyncApi) -> Result<(), BotError> {
        let send_chat_action_params = SendChatActionParams::builder()
            .chat_id((*((*message).chat)).id)
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
        self.process().await;

        Ok(())
    }

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }
}
