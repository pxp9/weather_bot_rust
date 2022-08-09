SELECT name , country , state , lon , lat FROM cities WHERE UPPER(name) = UPPER($1) AND UPPER(country) = UPPER($2) AND UPPER(state) = UPPER($3)
