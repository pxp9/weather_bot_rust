pub mod command;
pub mod db;
pub mod deliver;
pub mod open_weather_map;
pub mod seeds;
pub mod telegram;
pub mod workers;

use crate::db::BotDbError;
use crate::open_weather_map::client::ClientError;
use crate::telegram::client::ApiError;
use lazy_static::lazy_static;
use thiserror::Error;

lazy_static! {
    pub static ref RUST_TELEGRAM_BOT_TOKEN: String =
        std::env::var("RUST_TELEGRAM_BOT_TOKEN").expect("RUST_TELEGRAM_BOT_TOKEN not set");
    pub static ref OPEN_WEATHER_MAP_API_TOKEN: String =
        std::env::var("OPEN_WEATHER_MAP_API_TOKEN").expect("OPEN_WEATHER_MAP_API_TOKEN not set");
    pub static ref DATABASE_URL: String =
        std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
}

#[derive(Debug, Error)]
pub enum BotError {
    #[error(transparent)]
    MessageError(#[from] std::fmt::Error),
    #[error("Update can not be processed {}", self)]
    UpdateNotMessage(String),
    #[error(transparent)]
    TelegramError(#[from] ApiError),
    #[error(transparent)]
    DbError(#[from] BotDbError),
    #[error(transparent)]
    WeatherApiError(#[from] ClientError),
}
