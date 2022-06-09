
## MIT
### The MIT License
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)



# WeatherBot

This bot gives you weather info about any city in the world !

You have three commands to do:


- /weather_city
- /help
- /start

you can especify a country like this:

Madrid,ES

the bot uses acronyms using the standard ISO 3166
https://en.wikipedia.org/wiki/List_of_ISO_3166_country_codes

## Dependencies

- pip install pyowm
- pip install python-telegram-bot
- pip install schedule( future dependecy)

## Run CMD

You will need to create 2 enviroment variables in linux is in this file */etc/environment*

- TELEGRAM_BOT_TOKEN=TOKEN OF THE BOT
- OPEN_WEATHER_MAP_API_TOKEN=TOKEN OF THE API

Maybe you need to reboot and then

Run in command line : *python main.py*

## 3rd Party Documentations

- Python Open Weather Map API: https://pypi.org/project/pyowm/
- Python telegram bot API: https://pypi.org/project/python-telegram-bot/
- Schedule library: https://pypi.org/project/schedule/

## Future functions

The bot will send a daily message of weather info if user activate the option
