use serde_json::Value;
//use std::collections::HashMap;
use std::fs;
use tokio_postgres::{Error, Transaction};
// Realmente lo suyo es meter las ciudades en la bbdd :D
pub async fn read_json_cities(transaction: &mut Transaction<'_>) -> Result<(), Error> {
    let content = fs::read_to_string("resources/city.list.json").unwrap();
    let dict: Value = serde_json::from_str(&content).unwrap();
    for value in dict.as_array().unwrap().to_vec() {
        let obj = value.as_object().unwrap();
        let name = obj.get("name").unwrap().as_str().unwrap();
        let c = obj.get("country").unwrap().as_str().unwrap();
        let state = obj.get("state").unwrap().as_str().unwrap();
        let coords = obj.get("coord").unwrap().as_object().unwrap();
        let lon = coords.get("lon").unwrap().as_f64().unwrap();
        let lat = coords.get("lat").unwrap().as_f64().unwrap();
        let n = transaction
            .execute(
                "SELECT * FROM cities WHERE name = $1 AND country = $2 AND state = $3",
                &[&name, &c, &state],
            )
            .await?;
        if n == 0 {
            transaction.execute("INSERT INTO cities (name , country , state , lon , lat ) VALUES ($1 , $2 , $3 , $4 , $5)"
                , &[&name, &c,&state , &lon, &lat]).await?;
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
