UPDATE forecasts SET cron_expression = $2, next_delivery_at = $3 WHERE id = $1
