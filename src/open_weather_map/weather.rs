use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Coord {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct City {
    pub name: String,
    pub state: String,
    pub country: String,
    pub coords: Coord,
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
