use crate::db::{BotDbError, Repo};
use crate::open_weather_map::weather::City;
use serde_json::Value;
use std::fs;

pub async fn read_json_cities() -> Result<(), BotDbError> {
    let repo = Repo::new().await?;
    let connection = repo.pool.get().await?;

    // check if cities are in db
    let n = connection
        .execute("SELECT * FROM cities", &[])
        .await
        .unwrap();

    if n == 0 {
        // read json as file
        let content = fs::read_to_string("resources/city.list.json").unwrap();

        // Deserialize json with struct City defined open_weather_map::weather
        let cities = serde_json::from_str::<Vec<City>>(&content).unwrap();

        // For each city check if it is in db, if not is in db, insert the city
        for city in cities {
            let n = connection
                .execute(
                    "SELECT * FROM cities WHERE name = $1 AND country = $2 AND state = $3",
                    &[&city.name, &city.country, &city.state],
                )
                .await?;
            if n == 0 {
                connection.execute("INSERT INTO cities (name , country , state , lon , lat ) VALUES ($1 , $2 , $3 , $4 , $5)"
                    , &[&city.name, &city.country,&city.state , &city.coords.lon, &city.coords.lat]).await?;
            }
        }
    }
    Ok(())
}

pub async fn parse_weather(response: String) -> Result<String, ()> {
    let json: Value = serde_json::from_str(&response).unwrap();
    let dict = json.as_object().unwrap();
    let weather_desc = dict["weather"].as_array().unwrap()[0].as_object().unwrap()["description"]
        .as_str()
        .unwrap()
        .to_string();
    let main_info = dict["main"].as_object().unwrap();
    let temp = main_info["temp"].as_f64().unwrap();
    let temp_min = main_info["temp_min"].as_f64().unwrap();
    let temp_max = main_info["temp_max"].as_f64().unwrap();
    let pressure = main_info["pressure"].as_i64().unwrap();
    let humidity = main_info["humidity"].as_i64().unwrap();
    let st: String = format!(
        "\n🌍🌍 Weather: {}\n🌡️🌡️ Temp: {}ºC\n🧊🧊 Temp mínima: {} \n🔥🔥 Temp máxima: {} ºC\n⛰️⛰️ Presión: {} hPa\n💧💧 Humedad: {} %",
        weather_desc, temp, temp_min, temp_max, pressure, humidity
    );
    Ok(st)
}
