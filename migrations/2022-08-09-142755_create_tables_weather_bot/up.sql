-- Your SQL goes here

-- WEATHER BOT TABLES

CREATE TYPE client_state AS ENUM ('initial', 'set_city', 'find_city' , 'find_city_number' , 'set_city_number', 'time', 'offset');

-- for trigram index
CREATE EXTENSION IF NOT EXISTS pg_trgm;


CREATE TABLE cities (
  id SERIAL,
  name VARCHAR(80) NOT NULL,
  country VARCHAR(80) NOT NULL,
  state VARCHAR(80) NOT NULL,
  lon DOUBLE PRECISION NOT NULL,
  lat DOUBLE PRECISION NOT NULL,
  UNIQUE(name, country, state),
  PRIMARY KEY (id)
);

CREATE INDEX cities_name_trgm_idx ON cities USING gin (name gin_trgm_ops);


CREATE TABLE chats (
  id BIGINT,
  user_id BYTEA,
  state client_state DEFAULT 'initial' NOT NULL, -- Initial
  selected VARCHAR(80),
  default_city_id INT,
  PRIMARY KEY (id, user_id),
  CONSTRAINT fk_cities FOREIGN KEY(default_city_id) REFERENCES cities(id)
);

CREATE TABLE forecasts (
  chat_id BIGINT,
  city_id INT,
  cron_expression VARCHAR(80),
  last_delivered_at TIMESTAMP WITH TIME ZONE,
  next_delivery_at TIMESTAMP WITH TIME ZONE NOT NULL,
  update_at TIMESTAMP WITH TIME ZONE NOT NULL,
  create_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  PRIMARY KEY(chat_id, city_id),
  CONSTRAINT fk_cities FOREIGN KEY(city_id) REFERENCES cities(id),
  CONSTRAINT fk_chat FOREIGN KEY(chat_id) REFERENCES chats(id)
);
