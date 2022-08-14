-- Your SQL goes here

-- WEATHER BOT TABLES

CREATE TYPE client_state AS ENUM ('initial', 'set_city', 'find_city' , 'number');

CREATE TABLE chat (
  id BIGINT , 
  user_id BYTEA , 
  "user" BYTEA NOT NULL, -- Bytea in order to store it encrypted
  state client_state DEFAULT 'initial' NOT NULL, -- Initial
  before_state client_state DEFAULT 'initial' NOT NULL, -- Initial
  selected VARCHAR(80),
  city VARCHAR(80), 
  PRIMARY KEY (id , user_id)
);

CREATE TABLE cities (
  name VARCHAR(80),
  country VARCHAR(80),
  state VARCHAR(80),
  lon DOUBLE PRECISION, 
  lat DOUBLE PRECISION,
  PRIMARY KEY (name, country , state)
);
