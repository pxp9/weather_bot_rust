use serde_json::Value;
use std::collections::HashMap;
use std::fs;
// Implementacion inicial con Vec<Value>
// lo suyo seria pasarlo a un HashMap<(String , String) , (f64 , f64 , String , String)>
pub fn read_json_cities() -> HashMap<(String, String, String), (f64, f64, String, String, String)> {
    let content = fs::read_to_string("resources/city.list.json").unwrap();
    let dict: Value = serde_json::from_str(&content).unwrap();
    let mut map: HashMap<(String, String, String), (f64, f64, String, String, String)> =
        HashMap::new();
    for value in dict.as_array().unwrap().to_vec() {
        let obj = value.as_object().unwrap();
        let name = obj.get("name").unwrap().as_str().unwrap();
        let c = obj.get("country").unwrap().as_str().unwrap();
        let state = obj.get("state").unwrap().as_str().unwrap();
        let coords = obj.get("coord").unwrap().as_object().unwrap();
        let lon = coords.get("lon").unwrap().as_f64().unwrap();
        let lat = coords.get("lat").unwrap().as_f64().unwrap();
        map.insert(
            (name.to_uppercase(), c.to_uppercase(), state.to_uppercase()),
            (lon, lat, name.to_string(), c.to_string(), state.to_string()),
        );
    }
    map
}
// mejorar la busqueda
pub async fn search_city(
    city_name: String,
    country: String,
    state: String,
    map: &HashMap<(String, String, String), (f64, f64, String, String, String)>,
) -> Result<(f64, f64, String, String, String), ()> {
    match map.get(&(
        city_name.to_uppercase(),
        country.to_uppercase(),
        state.to_uppercase(),
    )) {
        Some(val) => Ok(val.clone()),
        None => Err(()),
    }
}

pub async fn parse_weather(response: String) -> Result<String, ()> {
    let json: Value = serde_json::from_str(&response).unwrap();
    let dict = json.as_object().unwrap();
    // gesationar mejor sir
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
        "\n🌍🌍 Weather: {}\n🌡️🌡️ Temp: {}ºC\n🧊🧊 Temp mínima: {} ºC\n🌡️🌡️ Temp máxima: {} ºC\n⛰️⛰️ Presión: {} hPa\n💧💧 Humedad: {} %",
        weather_desc, temp, temp_min, temp_max, pressure, humidity
    );
    Ok(st)
}
