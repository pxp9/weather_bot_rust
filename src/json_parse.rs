use crate::db::{BotDbError, Repo};
use crate::open_weather_map::weather::CityDeserialize;
use std::fs;

pub async fn read_json_cities() -> Result<(), BotDbError> {
    log::info!("Parsing cities from json");
    let repo = Repo::new().await?;
    let connection = repo.pool.get().await?;

    // check if cities are in db
    let n = connection
        .execute("SELECT * FROM cities LIMIT 1", &[])
        .await
        .unwrap();

    if n == 0 {
        // read json as file
        let content = fs::read_to_string("resources/city.list.json").unwrap();

        // Deserialize json with struct City defined open_weather_map::weather
        let cities = serde_json::from_str::<Vec<CityDeserialize>>(&content).unwrap();

        // For each city check if it is in db, if not is in db, insert the city
        for city in cities {
            let n = connection
                .execute(
                    "SELECT * FROM cities WHERE name = $1 AND country = $2 AND state = $3 LIMIT 1",
                    &[&city.name, &city.country, &city.state],
                )
                .await?;
            if n == 0 {
                connection.execute("INSERT INTO cities (name , country , state , lon , lat ) VALUES ($1 , $2 , $3 , $4 , $5)"
                    , &[&city.name, &city.country,&city.state , &city.coord.lon, &city.coord.lat]).await?;
            }
        }
    }
    log::info!("Cities are in database");
    Ok(())
}
