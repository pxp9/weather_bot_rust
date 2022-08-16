#!/usr/bin/env sh

    echo "Setting the database"
    ./diesel database setup

    ./weather_bot_rust
