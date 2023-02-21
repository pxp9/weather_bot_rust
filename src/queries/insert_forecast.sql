INSERT INTO forecasts (chat_id, user_id, city_id, cron_expression, next_delivery_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *
