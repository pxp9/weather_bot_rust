SELECT id , name , country , state, lon, lat  FROM cities WHERE UPPER(name) LIKE $1 ORDER BY name , country , state
