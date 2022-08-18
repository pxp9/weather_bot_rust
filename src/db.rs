use crate::open_weather_map::City;
use crate::open_weather_map::Coord;
use crate::seeds::SeedCity;
use crate::DATABASE_URL;
use bb8_postgres::bb8::Pool;
use bb8_postgres::bb8::RunError;
use bb8_postgres::tokio_postgres::tls::NoTls;
use bb8_postgres::tokio_postgres::Row;
use bb8_postgres::PostgresConnectionManager;
use postgres_types::{FromSql, ToSql};
use std::include_str;
use thiserror::Error;
use typed_builder::TypedBuilder;

const DELETE_CLIENT: &str = include_str!("queries/delete_client.sql");
const GET_CITY_BY_PATTERN: &str = include_str!("queries/get_city_by_pattern.sql");
const INSERT_CLIENT: &str = include_str!("queries/insert_client.sql");
const INSERT_CITY: &str = include_str!("queries/insert_city.sql");
const CHECK_USER_EXISTS: &str = include_str!("queries/check_user_exists.sql");
const CHECK_CITIES_EXIST: &str = include_str!("queries/check_cities_exist.sql");
const MODIFY_CITY: &str = include_str!("queries/modify_city.sql");
const MODIFY_BEFORE_STATE: &str = include_str!("queries/modify_before_state.sql");
const MODIFY_SELECTED: &str = include_str!("queries/modify_selected.sql");
const MODIFY_STATE: &str = include_str!("queries/modify_state.sql");
const SEARCH_CITY: &str = include_str!("queries/search_city.sql");
const SEARCH_CITY_BY_ID: &str = include_str!("queries/search_city_by_id.sql");
const GET_CHAT: &str = include_str!("queries/get_chat.sql");

#[derive(Debug, Error)]
pub enum BotDbError {
    #[error(transparent)]
    PoolError(#[from] RunError<bb8_postgres::tokio_postgres::Error>),
    #[error(transparent)]
    PgError(#[from] bb8_postgres::tokio_postgres::Error),
    #[error("City not found")]
    CityNotFoundError,
}

#[derive(Debug, Eq, PartialEq, Clone, ToSql, FromSql)]
#[postgres(name = "client_state")]
pub enum ClientState {
    #[postgres(name = "initial")]
    Initial,
    #[postgres(name = "find_city")]
    FindCity,
    #[postgres(name = "number")]
    Number,
    #[postgres(name = "set_city")]
    SetCity,
}

#[derive(Debug, Clone)]
pub struct Repo {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct Chat {
    pub id: i64,
    pub user_id: u64,
    pub state: ClientState,
    pub before_state: ClientState,
    pub selected: Option<String>,
    pub default_city_id: Option<i32>,
}

impl Repo {
    async fn pool(url: &str) -> Result<Pool<PostgresConnectionManager<NoTls>>, BotDbError> {
        let pg_mgr = PostgresConnectionManager::new_from_stringlike(url, NoTls)?;

        Ok(Pool::builder().build(pg_mgr).await?)
    }

    pub async fn new() -> Result<Self, BotDbError> {
        let pl = Self::pool(&DATABASE_URL).await?;
        Ok(Repo { pool: pl })
    }

    pub async fn check_user_exists(&self, chat_id: &i64, user_id: u64) -> Result<bool, BotDbError> {
        let connection = self.pool.get().await?;
        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(CHECK_USER_EXISTS, &[chat_id, &bytes])
            .await?;
        Ok(n == 1)
    }

    pub async fn find_or_create_chat(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<Chat, BotDbError> {
        if self.check_user_exists(chat_id, user_id).await? {
            let chat = self.get_chat(chat_id, user_id).await?;

            Ok(chat)
        } else {
            self.insert_client(chat_id, user_id).await?;
            let chat = self.get_chat(chat_id, user_id).await?;

            Ok(chat)
        }
    }

    pub async fn check_cities_exist(&self) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;
        let n = connection.execute(CHECK_CITIES_EXIST, &[]).await?;
        Ok(n)
    }

    pub async fn insert_city(&self, city: SeedCity) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let n = connection
            .execute(
                INSERT_CITY,
                &[
                    &city.name,
                    &city.country,
                    &city.state,
                    &city.coord.lon,
                    &city.coord.lat,
                ],
            )
            .await?;
        Ok(n)
    }

    pub async fn search_city(
        &self,
        name: &str,
        country: &str,
        state: &str,
    ) -> Result<City, BotDbError> {
        let connection = self.pool.get().await?;

        let vec: Vec<Row> = connection
            .query(SEARCH_CITY, &[&name, &country, &state])
            .await?;
        if vec.len() == 1 {
            Ok(Self::record_to_city(&vec[0]))
        } else {
            Err(BotDbError::CityNotFoundError)
        }
    }

    pub async fn search_city_by_id(&self, id: &i32) -> Result<City, BotDbError> {
        let connection = self.pool.get().await?;

        let vec: Vec<Row> = connection.query(SEARCH_CITY_BY_ID, &[id]).await?;
        if vec.len() == 1 {
            Ok(Self::record_to_city(&vec[0]))
        } else {
            Err(BotDbError::CityNotFoundError)
        }
    }

    pub async fn get_client_selected(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<Option<String>, BotDbError> {
        let chat = self.get_chat(chat_id, user_id).await?;

        Ok(chat.selected)
    }

    pub async fn get_client_default_city_id(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<Option<i32>, BotDbError> {
        let chat = self.get_chat(chat_id, user_id).await?;

        Ok(chat.default_city_id)
    }

    pub async fn get_client_before_state(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<ClientState, BotDbError> {
        let chat = self.get_chat(chat_id, user_id).await?;

        Ok(chat.before_state)
    }

    pub async fn get_client_state(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<ClientState, BotDbError> {
        let chat = self.get_chat(chat_id, user_id).await?;

        Ok(chat.state)
    }

    pub async fn get_chat(&self, chat_id: &i64, user_id: u64) -> Result<Chat, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let row = connection.query_one(GET_CHAT, &[chat_id, &bytes]).await?;

        let chat = Chat::builder()
            .id(row.get("id"))
            .user_id(user_id)
            .state(row.get("state"))
            .before_state(row.get("before_state"))
            .selected(row.try_get("selected").ok())
            .default_city_id(row.try_get("default_city_id").ok())
            .build();

        Ok(chat)
    }

    pub async fn insert_client(&self, chat_id: &i64, user_id: u64) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(
                INSERT_CLIENT,
                &[
                    chat_id,
                    &ClientState::Initial,
                    &ClientState::Initial,
                    &bytes,
                ],
            )
            .await?;
        Ok(n)
    }

    pub async fn delete_client(&self, chat_id: &i64, user_id: u64) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(DELETE_CLIENT, &[chat_id, &bytes])
            .await?;
        Ok(n)
    }

    pub async fn modify_state(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_state: ClientState,
    ) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(MODIFY_STATE, &[&new_state, chat_id, &bytes])
            .await?;
        Ok(n)
    }

    pub async fn modify_before_state(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_state: ClientState,
    ) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(MODIFY_BEFORE_STATE, &[&new_state, chat_id, &bytes])
            .await?;

        Ok(n)
    }

    pub async fn modify_default_city(
        &self,
        chat_id: &i64,
        user_id: u64,
        city_id: &i32,
    ) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(MODIFY_CITY, &[&city_id, chat_id, &bytes])
            .await?;

        Ok(n)
    }

    pub async fn modify_selected(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_selected: String,
    ) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(MODIFY_SELECTED, &[&new_selected, chat_id, &bytes])
            .await?;

        Ok(n)
    }

    pub async fn get_city_by_pattern(&self, pattern: &str) -> Result<Vec<Row>, BotDbError> {
        let connection = self.pool.get().await?;

        let st = format!("%{}%", pattern.to_uppercase());

        let vec = connection.query(GET_CITY_BY_PATTERN, &[&st]).await?;
        Ok(vec)
    }

    pub async fn get_city_row(&self, city: &str, n: usize) -> Result<City, BotDbError> {
        let vec: Vec<Row> = self.get_city_by_pattern(city).await?;
        if n > vec.len() {
            return Err(BotDbError::CityNotFoundError);
        }

        Ok(Self::record_to_city(&vec[n - 1]))
    }

    pub fn record_to_city(record: &Row) -> City {
        let coord = Coord::builder()
            .lon(record.get("lon"))
            .lat(record.get("lat"))
            .build();

        City::builder()
            .id(record.get("id"))
            .name(record.get("name"))
            .country(record.get("country"))
            .state(record.get("state"))
            .coord(coord)
            .build()
    }
}
#[cfg(test)]
mod db_test {
    use crate::db::*;
    use bb8_postgres::tokio_postgres::Row;

    #[tokio::test]
    async fn test_modify_state() {
        // Pick a random user of the DB
        let db_controller = Repo::new().await.unwrap();
        let connection = db_controller.pool.get().await.unwrap();

        let n = db_controller.insert_client(&111111, 1111111).await.unwrap();

        assert_eq!(n, 1_u64);

        let row: &Row = &connection
            .query_one("SELECT * FROM chats LIMIT 1", &[])
            .await
            .unwrap();

        let chat_id: i64 = row.get("id");

        let bytes: &[u8] = row.get("user_id");
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        let user_id = as_u64_le(&arr);

        // testing modify state

        let n = db_controller
            .modify_state(&chat_id, user_id, ClientState::FindCity)
            .await
            .unwrap();

        assert_eq!(n, 1_u64);

        // testing get state
        let actual_state = db_controller
            .get_client_state(&chat_id, user_id)
            .await
            .unwrap();

        assert_eq!(actual_state, ClientState::FindCity);

        let n = db_controller
            .modify_state(&chat_id, user_id, ClientState::Initial)
            .await
            .unwrap();
        assert_eq!(n, 1_u64);

        let actual_state = db_controller
            .get_client_state(&chat_id, user_id)
            .await
            .unwrap();

        assert_eq!(actual_state, ClientState::Initial);

        let n = db_controller.delete_client(&111111, 1111111).await.unwrap();
        assert_eq!(n, 1_u64);
    }
    fn as_u64_le(array: &[u8; 8]) -> u64 {
        (array[0] as u64)
            + ((array[1] as u64) << 8)
            + ((array[2] as u64) << 16)
            + ((array[3] as u64) << 24)
            + ((array[4] as u64) << 32)
            + ((array[5] as u64) << 40)
            + ((array[6] as u64) << 48)
            + ((array[7] as u64) << 56)
    }
}
