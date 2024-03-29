use crate::RUST_TELEGRAM_BOT_TOKEN;
use fang::FangError;
use frankenstein::AllowedUpdate;
use frankenstein::AsyncApi;
use frankenstein::AsyncTelegramApi;
use frankenstein::ChatAction;
use frankenstein::GetUpdatesParams;
use frankenstein::Message;
use frankenstein::MethodResponse;
use frankenstein::ParseMode;
use frankenstein::SendChatActionParams;
use frankenstein::SendMessageParams;
use frankenstein::Update;
use std::collections::VecDeque;
use thiserror::Error;
use tokio::sync::OnceCell;

static API_CLIENT: OnceCell<ApiClient> = OnceCell::const_new();

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    FrankensteinError(#[from] frankenstein::Error),
}

impl From<ApiError> for FangError {
    fn from(error: ApiError) -> Self {
        let description = format!("{:?}", error);

        FangError { description }
    }
}

#[derive(Clone)]
pub struct ApiClient {
    telegram_client: AsyncApi,
    update_params: GetUpdatesParams,
    buffer: VecDeque<Update>,
}

impl ApiClient {
    pub async fn api_client() -> &'static Self {
        API_CLIENT.get_or_init(ApiClient::new).await
    }

    pub async fn new() -> Self {
        let telegram_client = AsyncApi::new(&RUST_TELEGRAM_BOT_TOKEN);

        let update_params = GetUpdatesParams::builder()
            .allowed_updates(vec![AllowedUpdate::Message, AllowedUpdate::ChannelPost])
            .build();

        let buffer = VecDeque::new();

        Self {
            telegram_client,
            update_params,
            buffer,
        }
    }

    pub async fn next_update(&mut self) -> Option<Update> {
        if let Some(update) = self.buffer.pop_front() {
            return Some(update);
        }

        match self.telegram_client.get_updates(&self.update_params).await {
            Ok(updates) => {
                for update in updates.result {
                    self.buffer.push_back(update);
                }

                if let Some(last_update) = self.buffer.back() {
                    self.update_params.offset = Some((last_update.update_id + 1).into());
                }

                self.buffer.pop_front()
            }

            Err(err) => {
                log::error!("Failed to fetch updates {:?}", err);
                None
            }
        }
    }

    pub async fn send_typing(&self, chat_id: i64) -> Result<MethodResponse<bool>, ApiError> {
        let send_chat_action_params = SendChatActionParams::builder()
            .chat_id(chat_id)
            .action(ChatAction::Typing)
            .build();

        Ok(self
            .telegram_client
            .send_chat_action(&send_chat_action_params)
            .await?)
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        message_id: i32,
        text: String,
    ) -> Result<MethodResponse<Message>, ApiError> {
        let send_message_params = SendMessageParams::builder()
            .chat_id(chat_id)
            .text(text)
            .reply_to_message_id(message_id)
            .parse_mode(ParseMode::Html)
            .build();

        Ok(self
            .telegram_client
            .send_message(&send_message_params)
            .await?)
    }

    pub async fn send_message_without_reply(
        &self,
        chat_id: i64,
        text: String,
    ) -> Result<MethodResponse<Message>, ApiError> {
        let send_message_params = SendMessageParams::builder()
            .chat_id(chat_id)
            .text(text)
            .parse_mode(ParseMode::Html)
            .build();

        Ok(self
            .telegram_client
            .send_message(&send_message_params)
            .await?)
    }
}
