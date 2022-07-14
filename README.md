
## MIT
### The MIT License
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)



# Weather_bot_Rust

This bot gives you weather info about any city in the world !

You have a few commands to do:

- /start
- /city
- /pattern
- /cancel
- /set_city
- /set_search
- /default

you can especify a city and country like this:

Madrid,ES

or 

New York,US,NY

Spaces between comas and names does not mattter as well capital letters or not.

The bot uses acronyms using the standard ISO 3166
https://en.wikipedia.org/wiki/List_of_ISO_3166_country_codes

Also you can try pattern search which is easier than formatted search.

Just write a city name like this:

Madrid

The bot is going to answer:

```
1. Barajas de Madrid,ES
2. Comunidad de Madrid,ES
3. General La Madrid,AR
4. Humanes de Madrid,ES
5. Lamadrid,ES
6. Las Rozas de Madrid,ES
7. Madrid,CO
8. Madrid,ES
9. Madrid,MX
10. Madrid,PH
11. Madrid,US,IA
12. Madridanos,ES
13. Madridejos,ES
14. Madridejos,PH
15. New Madrid,US,MO
16. Partido de General La Madrid,AR
17. Provincia de Madrid,ES
18. Rivas-Vaciamadrid,ES
19. Valmadrid,ES
```
Then choose a number and get weather info.



## Dependencies

- frankenstein = {version = "0.18", default-features = false, features = ["async-http-client" , "async-telegram-trait"]}
- futures = "0.3.21"
- serde_json = "1.0"
- reqwest = "0.9.18"
- tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
- tokio-postgres = "0.7.6"
- openssl = "0.10.38"

## Run the bot
### Setup Postgres Database with *setup.sql* file in [resources](https://github.com/pxp9/weather_bot_rust/tree/master/resources)

### You will need to create 2 enviroment variables in linux is in this file */etc/environment*

- RUST_TELEGRAM_BOT_TOKEN=TOKEN OF THE BOT
- OPEN_WEATHER_MAP_API_TOKEN=TOKEN OF THE API

### Maybe you need to reboot or source */etc/environment* file 

### Also you need to setup a key.pem file that contains a private key in order to encrypt data in the db and move the file to resources.
```
$ openssl genrsa -out key.pem 2048
$ mv key.pem resources
```

Run in command line : *cargo run*
```
$ cargo run
```

## 3rd Party Documentations

- Open Weather Map API: https://openweathermap.org/current
- Rust telegram bot API: https://docs.rs/frankenstein/
- Json parser: https://docs.rs/serde_json/latest/serde_json/
- Async Http Request: https://docs.rs/reqwest/latest/reqwest/
- Async runtime required by telegram-bot : https://docs.rs/tokio/latest/tokio/
- Async database wrapper for PosgreSQL : https://docs.rs/tokio-postgres/
- OpenSSL oficial library for Rust for Encryption : https://docs.rs/openssl/
<!---
## Future functions

The bot will send a daily message of weather info if user activate the option
-->

