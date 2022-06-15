use serde_json::Value;
use std::fs;

pub fn read_json_cities() -> Vec<Value> {
    let content = fs::read_to_string("resources/city.list.json").unwrap();
    let dict: Value = serde_json::from_str(&content).unwrap();
    dict.as_array().unwrap().to_vec()
    //let elem = &vec[vec.len() - 1];
    //let obj = elem.as_object().unwrap();

    // mejorar el metodo de busqueda xd
}
pub async fn search_city(
    city_name: String,
    country: String,
    vec: &Vec<Value>,
) -> Result<(f64, f64), ()> {
    let mut found: bool = false;
    let mut lon: f64 = 0.0;
    let mut lat: f64 = 0.0;
    for value in vec {
        let obj = value.as_object().unwrap();
        let name = obj.get("name").unwrap().as_str().unwrap();
        let c = obj.get("country").unwrap().as_str().unwrap();
        if name == city_name && c == country {
            found = true;
            let coords = obj.get("coord").unwrap().as_object().unwrap();
            lon = coords.get("lon").unwrap().as_f64().unwrap();
            lat = coords.get("lat").unwrap().as_f64().unwrap();

            break;
        }
    }
    if found {
        Ok((lon, lat))
    } else {
        Err(())
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
    Ok(weather_desc)
}
