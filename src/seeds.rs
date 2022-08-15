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
    log::info!("Parsing cities from json");
    let repo = Repo::new().await?;

    // check if cities are in db
    let n = repo.check_cities_exist().await?;

    if n == 0 {
        // read json as file
        let content = fs::read_to_string("city.list.json").unwrap();

        // Deserialize json with struct City defined open_weather_map::weather
        let cities = serde_json::from_str::<Vec<SeedCity>>(&content).unwrap();

        // For each city check if it is in db, if not is in db, insert the city
        for city in cities {
            if let Err(err) = repo.insert_city(city).await {
                log::error!("Duplicated entry, not unique: {:?}", err);
            }
        }
    }
    log::info!("Cities are in database");
    Ok(())
}
