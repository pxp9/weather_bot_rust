SELECT 'CREATE DATABASE weather_bot'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'weather_bot')\gexec
