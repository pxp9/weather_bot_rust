SELECT 'CREATE DATABASE weather_bot'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'weather_bot')\gexec

-- CREATE DATABASE IF NOT EXISTS weather_bot ;
-- ALTER TABLE chat ALTER COLUMN id TYPE BIGINT;
-- openssl genrsa -out key.pem 2048
\c weather_bot --USE weather_bot;
CREATE TABLE IF NOT EXISTS chat (
  id BIGINT , 
  user_id BYTEA , 
  "user" BYTEA NOT NULL, -- Bytea in order to store it encrypted
  state VARCHAR(20) NOT NULL, -- Initial
  before_state VARCHAR(20) NOT NULL, -- Initial
  selected VARCHAR(80),
  city VARCHAR(80), 
  pattern_search BOOLEAN,
  PRIMARY KEY (id , user_id)
);
-- UPDATE chat SET state = 'Initial';
-- SELECT name , country , state , lon , lat FROM chat WHERE UPPER(name) = UPPER('Madrid') AND UPPER(country) = UPPER('ES') 
-- AND UPPER(state) = UPPER('') ;
CREATE TABLE IF NOT EXISTS cities (
  name VARCHAR(80),
  country VARCHAR(80),
  state VARCHAR(80),
  lon DOUBLE PRECISION, 
  lat DOUBLE PRECISION,
  PRIMARY KEY (name, country , state)
);
