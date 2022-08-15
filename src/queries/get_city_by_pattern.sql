SELECT id , name , country , state  FROM cities WHERE UPPER(name) LIKE $1 ORDER BY name , country , state
