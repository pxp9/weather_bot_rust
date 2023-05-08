db:
	docker run --rm -d --name postgres -p 5432:5432 \
  -e POSTGRES_DB=weather_bot \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  postgres:14.5

run:
	RUST_LOG=info DATABASE_URL=postgres://postgres:postgres@localhost/weather_bot cargo run

diesel:
	DATABASE_URL=postgres://postgres:postgres@localhost/weather_bot diesel migration run
stop: 
	docker kill postgres

docker_run:
	docker run --rm --env-file ./.env --network host --name weather_bot -t pxp9/weather_bot_rust:latest

compose:
	if [ -d "./target" ]; then \
		rm -r "./target"; \
	fi \
	
	if [ ! -d "./db-data" ]; then \
		mkdir "db-data"; \
	fi
	echo "Docker compose";
	docker compose up -d
