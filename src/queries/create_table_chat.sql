CREATE TABLE IF NOT EXISTS chat (
  id BIGINT , 
  user_id BYTEA , 
  "user" BYTEA NOT NULL, -- Bytea in order to store it encrypted
  state client_state DEFAULT 'initial' NOT NULL, -- Initial
  before_state client_state DEFAULT 'initial' NOT NULL, -- Initial
  selected VARCHAR(80),
  city VARCHAR(80), 
  PRIMARY KEY (id , user_id)
)
