use bb8_postgres::bb8::Pool;
use bb8_postgres::bb8::RunError;
use bb8_postgres::tokio_postgres::tls::NoTls;
use bb8_postgres::tokio_postgres::{Row, Transaction};
use bb8_postgres::PostgresConnectionManager;
use openssl::encrypt::{Decrypter, Encrypter};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Padding;
use postgres_types::{FromSql, ToSql};
use std::include_str;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotDbError {
    #[error(transparent)]
    PoolError(#[from] RunError<bb8_postgres::tokio_postgres::Error>),
    #[error(transparent)]
    PgError(#[from] bb8_postgres::tokio_postgres::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("City not found")]
    CityNotFoundError,
}

#[derive(Debug, Clone)]
pub struct DbController {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

#[derive(Debug, Eq, PartialEq, Clone, ToSql, FromSql)]
#[postgres(name = "client_state")]
pub enum ClientState {
    #[postgres(name = "initial")]
    Initial,
    #[postgres(name = "pattern")]
    Pattern,
    #[postgres(name = "number")]
    Number,
    #[postgres(name = "set_city")]
    SetCity,
}

const URL: &str = "postgres://postgres:postgres@localhost/weather_bot";
const CREATE_ENUM_TYPE: &str = include_str!("queries/create_enum_type.sql");
const CREATE_DB: &str = include_str!("queries/create_db.sql");
const DROP_DB: &str = include_str!("queries/drop_db.sql");
const CREATE_TABLE_CHAT: &str = include_str!("queries/create_table_chat.sql");
const CREATE_TABLE_CITIES: &str = include_str!("queries/create_table_cities.sql");
const DELETE_CLIENT: &str = include_str!("queries/delete_client.sql");
const GET_CITY_BY_PATTERN: &str = include_str!("queries/get_city_by_pattern.sql");
const INSERT_CLIENT: &str = include_str!("queries/insert_client.sql");
const IS_IN_DB: &str = include_str!("queries/is_in_db.sql");
const MODIFY_CITY: &str = include_str!("queries/modify_city.sql");
const MODIFY_BEFORE_STATE: &str = include_str!("queries/modify_before_state.sql");
const MODIFY_SELECTED: &str = include_str!("queries/modify_selected.sql");
const MODIFY_STATE: &str = include_str!("queries/modify_state.sql");
const SEARCH_CITY: &str = include_str!("queries/search_city.sql");
const SEARCH_CLIENT: &str = include_str!("queries/search_client.sql");

impl DbController {
    async fn migrate(db_transaction: &mut Transaction<'_>) -> Result<(), BotDbError> {
        db_transaction.execute(CREATE_ENUM_TYPE, &[]).await?;
        db_transaction.execute(CREATE_TABLE_CHAT, &[]).await?;
        db_transaction.execute(CREATE_TABLE_CITIES, &[]).await?;
        Ok(())
    }

    async fn pool(url: &str) -> Result<Pool<PostgresConnectionManager<NoTls>>, BotDbError> {
        let pg_mgr = PostgresConnectionManager::new_from_stringlike(url, NoTls).unwrap();

        Ok(Pool::builder().build(pg_mgr).await?)
    }

    pub async fn new() -> Result<Self, BotDbError> {
        let pl = Self::pool(URL).await?;
        Ok(DbController { pool: pl })
    }

    pub async fn setup_db() -> Result<(), BotDbError> {
        let pl = Self::pool("postgres://postgres:postgres@localhost").await?;
        let mut connection = pl.get().await?;
        let db_transaction = connection.transaction().await?;
        db_transaction.execute(CREATE_DB, &[]).await?;
        db_transaction.commit().await?;

        let pl = Self::pool(URL).await?;
        let mut connection = pl.get().await?;
        let mut db_transaction = connection.transaction().await?;
        Self::migrate(&mut db_transaction).await?;
        Ok(db_transaction.commit().await?)
    }

    async fn rollback(db_transaction: Transaction<'_>) -> Result<(), BotDbError> {
        Ok(db_transaction.rollback().await?)
    }

    async fn drop_db(db_transaction: Transaction<'_>) -> Result<(), BotDbError> {
        db_transaction.execute(DROP_DB, &[]).await?;
        Ok(db_transaction.commit().await?)
    }

    // Encrypt a String into a BYTEA
    async fn encrypt_string(some_string: String, keypair: &PKey<Private>) -> Vec<u8> {
        let mut encrypter = Encrypter::new(keypair).unwrap();
        encrypter.set_rsa_padding(Padding::PKCS1).unwrap();
        let st_bytes = some_string.as_bytes();
        let len: usize = encrypter.encrypt_len(st_bytes).unwrap();
        let mut encrypted = vec![0; len];
        let encrypted_len = encrypter.encrypt(st_bytes, &mut encrypted).unwrap();
        encrypted.truncate(encrypted_len);
        encrypted
    }

    // Decrypt a BYTEA into a String
    async fn decrypt_string(encrypted: &[u8], keypair: &PKey<Private>) -> String {
        let mut decrypter = Decrypter::new(keypair).unwrap();
        decrypter.set_rsa_padding(Padding::PKCS1).unwrap();
        let buffer_len = decrypter.decrypt_len(encrypted).unwrap();
        let mut decrypted = vec![0; buffer_len];
        let decrypted_len = decrypter.decrypt(encrypted, &mut decrypted).unwrap();
        decrypted.truncate(decrypted_len);
        String::from_utf8(decrypted).unwrap()
    }

    pub async fn is_in_db(&self, chat_id: &i64, user_id: u64) -> Result<bool, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;
        let bytes = user_id.to_le_bytes().to_vec();

        let n = db_transaction.execute(IS_IN_DB, &[chat_id, &bytes]).await?;
        db_transaction.commit().await?;
        Ok(n == 1)
    }

    pub async fn search_city(
        &self,
        n: &String,
        c: &String,
        s: &String,
    ) -> Result<(f64, f64, String, String, String), BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let vec: Vec<Row> = db_transaction.query(SEARCH_CITY, &[n, c, s]).await?;
        db_transaction.commit().await?;
        if vec.len() == 1 {
            Ok((
                vec[0].get("lon"),
                vec[0].get("lat"),
                vec[0].get("name"),
                vec[0].get("country"),
                vec[0].get("state"),
            ))
        } else {
            Err(BotDbError::CityNotFoundError)
        }
    }

    pub async fn get_client_selected(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<String, BotDbError> {
        let row: &Row = &self.search_client(chat_id, user_id).await?;

        Ok(row.get("selected"))
    }

    pub async fn get_client_city(&self, chat_id: &i64, user_id: u64) -> Result<String, BotDbError> {
        let row: &Row = &self.search_client(chat_id, user_id).await?;

        Ok(row.try_get("city")?)
    }

    pub async fn get_client_before_state(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<String, BotDbError> {
        let row: &Row = &self.search_client(chat_id, user_id).await?;

        Ok(row.get("before_state"))
    }
    pub async fn get_client_state(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<String, BotDbError> {
        let row: &Row = &self.search_client(chat_id, user_id).await?;

        Ok(row.get("state"))
    }

    pub async fn search_client(&self, chat_id: &i64, user_id: u64) -> Result<Row, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let row = db_transaction
            .query_one(SEARCH_CLIENT, &[chat_id, &bytes])
            .await?;

        db_transaction.commit().await?;
        Ok(row)
    }
    pub async fn insert_client(
        &self,
        chat_id: &i64,
        user_id: u64,
        user: String,
        keypair: &PKey<Private>,
    ) -> Result<u64, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();
        let user_encrypted = Self::encrypt_string(user, keypair).await;

        let n = db_transaction
            .execute(
                INSERT_CLIENT,
                &[
                    chat_id,
                    &user_encrypted,
                    &ClientState::Initial,
                    &ClientState::Initial,
                    &bytes,
                ],
            )
            .await?;
        db_transaction.commit().await?;
        Ok(n)
    }
    pub async fn delete_client(&self, chat_id: &i64, user_id: u64) -> Result<u64, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = db_transaction
            .execute(DELETE_CLIENT, &[chat_id, &bytes])
            .await?;
        db_transaction.commit().await?;
        Ok(n)
    }
    pub async fn modify_state(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_state: ClientState,
    ) -> Result<u64, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = db_transaction
            .execute(MODIFY_STATE, &[&new_state, chat_id, &bytes])
            .await?;
        db_transaction.commit().await?;
        Ok(n)
    }

    pub async fn modify_before_state(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_state: ClientState,
    ) -> Result<u64, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = db_transaction
            .execute(MODIFY_BEFORE_STATE, &[&new_state, chat_id, &bytes])
            .await?;

        db_transaction.commit().await?;
        Ok(n)
    }

    pub async fn modify_city(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_city: String,
    ) -> Result<u64, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = db_transaction
            .execute(MODIFY_CITY, &[&new_city, chat_id, &bytes])
            .await?;

        db_transaction.commit().await?;
        Ok(n)
    }
    pub async fn modify_selected(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_selected: String,
    ) -> Result<u64, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = db_transaction
            .execute(MODIFY_SELECTED, &[&new_selected, chat_id, &bytes])
            .await?;

        db_transaction.commit().await?;
        Ok(n)
    }

    pub async fn get_city_by_pattern(&self, city: &str) -> Result<Vec<Row>, BotDbError> {
        let mut connection = self.pool.get().await?;
        let db_transaction = connection.transaction().await?;

        let st = format!("%{}%", city.to_uppercase());

        let vec = db_transaction.query(GET_CITY_BY_PATTERN, &[&st]).await?;
        db_transaction.commit().await?;
        Ok(vec)
    }
    pub async fn get_city_row(
        &self,
        city: &str,
        n: usize,
    ) -> Result<(String, String, String), BotDbError> {
        let vec: Vec<Row> = self.get_city_by_pattern(city).await?;
        if n > vec.len() {
            return Err(BotDbError::CityNotFoundError);
        }
        Ok((
            vec[n - 1].get("name"),
            vec[n - 1].get("country"),
            vec[n - 1].get("state"),
        ))
    }
}
#[cfg(test)]
mod db_test {
    use crate::db::*;
    use std::convert::TryInto;
    use tokio_postgres::{NoTls, Row};

    #[tokio::test]
    async fn test_modify_state() {
        // Pick a random user of the DB
        let (mut client, connection) =
            tokio_postgres::connect("host=localhost dbname=weather_bot user=postgres", NoTls)
                .await
                .unwrap();
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        let mut transaction = client.transaction().await.unwrap();
        let row: &Row = &transaction
            .query("SELECT * FROM chat LIMIT 1", &[])
            .await
            .unwrap()[0];
        let chat_id: i64 = row.get("id");
        let bytes: &[u8] = row.get("user_id");
        let user_id: u64 = u64::from_be_bytes(bytes.try_into().expect("incorrect len"));
        // testing modify state
        let n = modify_state(
            &mut transaction,
            &chat_id,
            user_id,
            String::from("AskingCity"),
        )
        .await
        .unwrap();
        assert_eq!(n, 1_u64);
        transaction.commit().await.unwrap();
        let mut transaction = client.transaction().await.unwrap();

        // testing get state
        let actual_state = get_client_state(&mut transaction, &chat_id, user_id)
            .await
            .unwrap();
        assert_eq!(actual_state, String::from("AskingCity"));

        let n = modify_state(&mut transaction, &chat_id, user_id, String::from("Initial"))
            .await
            .unwrap();
        assert_eq!(n, 1_u64);
        transaction.commit().await.unwrap();

        let mut transaction = client.transaction().await.unwrap();
        let actual_state = get_client_state(&mut transaction, &chat_id, user_id)
            .await
            .unwrap();
        assert_eq!(actual_state, String::from("Initial"));
        transaction.commit().await.unwrap();
    }
}
