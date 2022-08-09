db:
	docker run --rm -d --name postgres -p 5432:5432 \
  -e POSTGRES_DB=weather_bot \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  postgres:latest
	
diesel:
	DATABASE_URL=postgres://postgres:postgres@localhost/weather_bot diesel migration run
stop: 
	docker kill postgres
