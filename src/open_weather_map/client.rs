use super::weather::Weather;
use super::weather::WeatherForecast;
use crate::OPEN_WEATHER_MAP_API_TOKEN;
use reqwest::Client;
use thiserror::Error;
use tokio::sync::OnceCell;
use typed_builder::TypedBuilder;

const UNITS: &str = "metric";
const LANG: &str = "en";

static WEATHER_CLIENT: OnceCell<WeatherApiClient> = OnceCell::const_new();

#[derive(TypedBuilder, Clone)]
pub struct WeatherApiClient {
    client: Client,
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
    pub async fn weather_client() -> &'static Self {
        WEATHER_CLIENT.get_or_init(WeatherApiClient::new).await
    }
    pub async fn new() -> Self {
        WeatherApiClient::builder().client(Client::new()).build()
    }
    pub async fn fetch(&self, lat: f64, lon: f64) -> Result<Weather, ClientError> {
        let request_url = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units={}&lang={}",
            lat, lon, OPEN_WEATHER_MAP_API_TOKEN.as_str(), UNITS, LANG
        );

        let response = self.client.get(&request_url).send()?;

        Self::decode_response(response)
    }

    pub async fn fetch_weekly(&self, lat: f64, lon: f64) -> Result<WeatherForecast, ClientError> {
        let request_url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid={}&units={}&lang={}&cnt={}",
            lat,
            lon,
            OPEN_WEATHER_MAP_API_TOKEN.as_str(),
            UNITS,
            LANG,
            9,
        );

        let response = self.client.get(&request_url).send()?;

        Self::decode_weekly_response(response)
    }

    pub fn decode_weekly_response(
        mut response: reqwest::Response,
    ) -> Result<WeatherForecast, ClientError> {
        let status_code = response.status().as_u16();
        let string_response = response.text()?;

        if status_code == 200 {
            let json_result: WeatherForecast = serde_json::from_str(&string_response).unwrap();
            return Ok(json_result);
        };

        Err(ClientError::StatusCodeError((status_code, string_response)))
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

mod tests {

    #[tokio::test]
    async fn fetch_weekly() {
        let weather_client = super::WeatherApiClient::weather_client().await;
        weather_client
            .fetch_weekly(-3.70256, 40.4165)
            .await
            .unwrap();
    }
}
