use crate::db::{BotDbError, Repo};
use crate::open_weather_map::weather::Coord;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SeedCity {
    pub name: String,
    pub state: String,
    pub country: String,
    pub coord: Coord,
}

pub async fn insert_seeds() -> Result<(), BotDbError> {
    let repo = Repo::new().await?;

    // check if cities are in db
    let n = repo.check_cities_exist().await?;

    if n == 0 {
        // read json as file
        log::info!("Reading cities from json");
        let content = fs::read_to_string("resources/city.list.json").unwrap();

        // Deserialize json with struct City defined open_weather_map::weather
        log::info!("Parsing cities from json");
        let cities = serde_json::from_str::<Vec<SeedCity>>(&content).unwrap();

        // For each city check if it is in db, if not is in db, insert the city
        log::info!("Inserting cities");
        for city in cities {
            repo.insert_city(city).await?;
        }

        log::info!("Cities are in database");
    } else {
        log::info!("Cities are already inserted, skipping");
    }

    Ok(())
}
