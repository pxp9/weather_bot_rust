
## MIT
### The MIT License
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)



# WeatherBot

This bot gives you weather info about any city in the world !

You have three commands to do:


- /city

<!--- 
- /help
- /start 
-->

you must especify a city and country like this:

Madrid,ES

the bot uses acronyms using the standard ISO 3166
https://en.wikipedia.org/wiki/List_of_ISO_3166_country_codes

## Dependencies

- telegram-bot = "0.7"
- futures = "0.3.21"
- serde_json = "1.0"
- reqwest = "0.9.18"
- tokio = { version = "0.2.22", features = ["full"] }


## Run CMD

You will need to create 2 enviroment variables in linux is in this file */etc/environment*

- RUST_TELEGRAM_BOT_TOKEN=TOKEN OF THE BOT
- OPEN_WEATHER_MAP_API_TOKEN=TOKEN OF THE API

Maybe you need to reboot and then or source */etc/environment* file

Run in command line : *cargo run*

## 3rd Party Documentations

- Open Weather Map API: https://openweathermap.org/current
- Rust telegram bot API: https://docs.rs/telegram-bot/latest/telegram_bot/
- Json parser: https://docs.rs/serde_json/latest/serde_json/
- Http Request: https://docs.rs/reqwest/latest/reqwest/
- Async runtime required by telegram-bot : https://docs.rs/tokio/latest/tokio/

<!---
## Future functions

The bot will send a daily message of weather info if user activate the option
-->

