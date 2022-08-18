SELECT id, state , default_city_id , selected , before_state FROM chats WHERE id = $1 AND user_id = $2
