db:
	docker run --rm -d --name postgres -p 5432:5432 \
  -e POSTGRES_DB=weather_bot \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  postgres:latest

run:
	RUST_LOG=info DATABASE_URL=postgres://postgres:postgres@localhost/weather_bot cargo run

diesel:
	DATABASE_URL=postgres://postgres:postgres@localhost/weather_bot diesel migration run
stop: 
	docker kill postgres

docker_image:
	docker run --env-file ./.env --network host -t pxp9/weather_bot_rust:latest
