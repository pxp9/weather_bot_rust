use crate::OPEN_WEATHER_MAP_API_TOKEN;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use typed_builder::TypedBuilder;

const UNITS: &str = "metric";
const LANG: &str = "en";

#[derive(TypedBuilder)]
pub struct WeatherApiClient {
    lat: i32,
    lon: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Weather {
    pub coord: Coord,
    pub weather: Vec<Coord>,
    pub base: String,
    pub main: Main,
    pub visibility: u32,
    pub wind: Wind,
    pub clouds: Clouds,
    pub dt: u32,
    pub timezone: i64,
    pub id: u32,
    pub name: String,
    pub code: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coord {
    pub lon: i64,
    pub lat: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherInfo {
    pub id: u32,
    pub main: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Main {
    pub temp: i64,
    pub feels_like: i64,
    pub temp_min: i64,
    pub temp_max: i64,
    pub pressue: u32,
    pub humidity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Wind {
    pub speed: i64,
    pub deg: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Clouds {
    pub all: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sys {
    #[serde(rename = "type")]
    pub sys_type: u32,
    pub message: i64,
    pub country: String,
    pub sunrise: u32,
    pub sunset: u32,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    DecodeError(#[from] serde_json::Error),
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error("invalid status code {}", self)]
    StatusCodeError((u16, String)),
}

impl WeatherApiClient {
    pub async fn fetch(&self) -> Result<Weather, ClientError> {
        let request_url = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units={}&lang={}",
            self.lat, self.lon, OPEN_WEATHER_MAP_API_TOKEN.as_str(), UNITS, LANG
        );

        let response = reqwest::get(&request_url)?;

        Self::decode_response(response)
    }

    pub fn decode_response(mut response: reqwest::Response) -> Result<Weather, ClientError> {
        let status_code = response.status().as_u16();
        let string_response = response.text()?;

        if status_code == 200 {
            let json_result: Weather = serde_json::from_str(&string_response)?;
            return Ok(json_result);
        };

        Err(ClientError::StatusCodeError((status_code, string_response)))
    }
}
