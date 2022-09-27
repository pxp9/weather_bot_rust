UPDATE forecasts SET last_delivered_at = $4, next_delivery_at = $5 WHERE chat_id = $1 AND user_id = $2 AND city_id = $3 RETURNING *
