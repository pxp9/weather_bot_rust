pub mod db;
pub mod json_parse;
pub mod telegram;

use lazy_static::lazy_static;
lazy_static! {
    pub static ref RUST_TELEGRAM_BOT_TOKEN: String =
        std::env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    pub static ref OPEN_WEATHER_MAP_API_TOKEN: String =
        std::env::var("OPEN_WEATHER_MAP_API_TOKEN").expect("OPEN_WEATHER_MAP_API_TOKEN not set");
    pub static ref BINARY_FILE: Vec<u8> =
        std::fs::read("./resources/key.pem").expect("resources/key.pem not set");
}

use crate::db::BotDbError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum BotError {
    #[error(transparent)]
    MessageError(#[from] std::fmt::Error),
    #[error(transparent)]
    TelegramError(#[from] frankenstein::Error),
    #[error(transparent)]
    DbError(#[from] BotDbError),
}
