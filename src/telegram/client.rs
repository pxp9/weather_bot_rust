use frankenstein::AllowedUpdate;
use frankenstein::AsyncApi;
use frankenstein::AsyncTelegramApi;
use frankenstein::Error;
use frankenstein::GetUpdatesParams;
use frankenstein::Message;
use frankenstein::ParseMode;
use frankenstein::SendMessageParams;
use frankenstein::Update;
use std::collections::VecDeque;
use std::env;

pub struct ApiClient {
    telegram_client: AsyncApi,
    update_params: GetUpdatesParams,
    buffer: VecDeque<Update>,
}
impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient {
    pub fn new() -> Self {
        let token = env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
        let telegram_client = AsyncApi::new(&token);

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
                    self.update_params.offset = Some(last_update.update_id + 1);
                }

                self.buffer.pop_front()
            }

            Err(err) => {
                log::error!("Failed to fetch updates {:?}", err);
                None
            }
        }
    }

    pub async fn send_message(&self, message: &Message, text: &str) -> Result<(), Error> {
        let send_message_params = SendMessageParams::builder()
            .chat_id(message.chat.id)
            .text(text)
            .reply_to_message_id(message.message_id)
            .parse_mode(ParseMode::Html)
            .build();

        self.telegram_client
            .send_message(&send_message_params)
            .await?;

        Ok(())
    }
}
