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
)
