<p align="center"><img src="logo.png" alt="logo" width="350px"></p>

## MIT
### The MIT License
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)



# Weather_bot_Rust

This bot provides you weather info about any city in the world !

You have a few commands to do:

- /start
- /find_city
- /cancel
- /set_default_city
- /default

Search by find_city command.

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

You can see them in [Cargo.toml](https://github.com/pxp9/weather_bot_rust/blob/master/Cargo.toml) file.


## Run the bot


### You will need to create these environment variables.

- RUST_TELEGRAM_BOT_TOKEN=TOKEN OF THE BOT
- OPEN_WEATHER_MAP_API_TOKEN=TOKEN OF THE API
- RUST_LOG=info
- DATABASE_URL=postgres://postgres:postgres@localhost/weather_bot

### Setup Postgres Database

- Install docker.

Start PostgreSQL container.
```
$ make db
```
Runs the migrations

```
$ make diesel
```

Run the bot

```
$ make run
```

Stop Docker PostgresSQL DB

```
$ make stop
```

## Run full bot with Docker compose 

- Install docker
- Install docker compose (tested with [docker-compose v2](https://docs.docker.com/compose/#compose-v2-and-the-new-docker-compose-command)).

### Set this environments values in `.env` file.

- SET_DB see in [start.sh](https://github.com/pxp9/weather_bot_rust/blob/master/docker/start.sh) file what it does.
- REVERT_DB see in [start.sh](https://github.com/pxp9/weather_bot_rust/blob/master/docker/start.sh) file what it does.
- OPEN_WEATHER_MAP_TOKEN=TOKEN
- RUST_TELEGRAM_BOT_TOKEN=TOKEN
- RUST_LOG=info
- DATABASE_URL=postgres://postgres:postgres@db/weather_bot

```
$ mkdir db-data
$ docker compose up
```

This will run both containers, PostgreSQL container and Bot container
## 3rd Party Documentations

- [Open Weather Map API](https://openweathermap.org/current)
- [Rust telegram bot API](https://docs.rs/frankenstein/)
- [Json parser](https://docs.rs/serde_json/latest/serde_json/)
- [Serialize Deserialize library](https://docs.rs/serde/latest/serde/)
- [Async Http Request](https://docs.rs/reqwest/latest/reqwest/)
- [Async runtime required by telegram-bot](https://docs.rs/tokio/latest/tokio/)
- [Async database wrapper for PosgreSQL with Pool](https://docs.rs/bb8-postgres/0.7.0/bb8_postgres/)
<!---
## Future functions

The bot will send a daily message of weather info if user activate the option
-->
