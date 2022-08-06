CREATE TABLE IF NOT EXISTS cities (
  name VARCHAR(80),
  country VARCHAR(80),
  state VARCHAR(80),
  lon DOUBLE PRECISION, 
  lat DOUBLE PRECISION,
  PRIMARY KEY (name, country , state)
)
