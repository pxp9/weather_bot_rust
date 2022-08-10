use crate::DATABASE_URL;
use bb8_postgres::bb8::Pool;
use bb8_postgres::bb8::RunError;
use bb8_postgres::tokio_postgres::tls::NoTls;
use bb8_postgres::tokio_postgres::Row;
use bb8_postgres::PostgresConnectionManager;
#[cfg(test)]
use openssl::encrypt::Decrypter;
use openssl::encrypt::Encrypter;
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
    EncryptError(#[from] openssl::error::ErrorStack),
    #[error(transparent)]
    StringError(#[from] std::string::FromUtf8Error),
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

const DELETE_CLIENT: &str = include_str!("queries/delete_client.sql");
const GET_CITY_BY_PATTERN: &str = include_str!("queries/get_city_by_pattern.sql");
const INSERT_CLIENT: &str = include_str!("queries/insert_client.sql");
const CHECK_USER_EXISTS: &str = include_str!("queries/check_user_exists.sql");
const MODIFY_CITY: &str = include_str!("queries/modify_city.sql");
const MODIFY_BEFORE_STATE: &str = include_str!("queries/modify_before_state.sql");
const MODIFY_SELECTED: &str = include_str!("queries/modify_selected.sql");
const MODIFY_STATE: &str = include_str!("queries/modify_state.sql");
const SEARCH_CITY: &str = include_str!("queries/search_city.sql");
const SEARCH_CLIENT: &str = include_str!("queries/search_client.sql");

impl DbController {
    async fn pool(url: &str) -> Result<Pool<PostgresConnectionManager<NoTls>>, BotDbError> {
        let pg_mgr = PostgresConnectionManager::new_from_stringlike(url, NoTls)?;

        Ok(Pool::builder().build(pg_mgr).await?)
    }

    pub async fn new() -> Result<Self, BotDbError> {
        let pl = Self::pool(&DATABASE_URL).await?;
        Ok(DbController { pool: pl })
    }

    // Encrypt a String into a BYTEA
    fn encrypt_string(some_string: String, keypair: &PKey<Private>) -> Result<Vec<u8>, BotDbError> {
        let mut encrypter = Encrypter::new(keypair)?;
        encrypter.set_rsa_padding(Padding::PKCS1)?;
        let st_bytes = some_string.as_bytes();
        let len: usize = encrypter.encrypt_len(st_bytes)?;
        let mut encrypted = vec![0; len];
        let encrypted_len = encrypter.encrypt(st_bytes, &mut encrypted)?;
        encrypted.truncate(encrypted_len);
        Ok(encrypted)
    }

    // Decrypt a BYTEA into a String
    #[cfg(test)]
    fn decrypt_string(encrypted: &[u8], keypair: &PKey<Private>) -> Result<String, BotDbError> {
        let mut decrypter = Decrypter::new(keypair)?;
        decrypter.set_rsa_padding(Padding::PKCS1)?;
        let buffer_len = decrypter.decrypt_len(encrypted)?;
        let mut decrypted = vec![0; buffer_len];
        let decrypted_len = decrypter.decrypt(encrypted, &mut decrypted)?;
        decrypted.truncate(decrypted_len);
        Ok(String::from_utf8(decrypted)?)
    }

    pub async fn check_user_exists(&self, chat_id: &i64, user_id: u64) -> Result<bool, BotDbError> {
        let connection = self.pool.get().await?;
        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(CHECK_USER_EXISTS, &[chat_id, &bytes])
            .await?;
        Ok(n == 1)
    }

    pub async fn search_city(
        &self,
        n: &String,
        c: &String,
        s: &String,
    ) -> Result<(f64, f64, String, String, String), BotDbError> {
        let connection = self.pool.get().await?;

        let vec: Vec<Row> = connection.query(SEARCH_CITY, &[n, c, s]).await?;
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
    ) -> Result<ClientState, BotDbError> {
        let row: &Row = &self.search_client(chat_id, user_id).await?;

        Ok(row.get("before_state"))
    }
    pub async fn get_client_state(
        &self,
        chat_id: &i64,
        user_id: u64,
    ) -> Result<ClientState, BotDbError> {
        let row: &Row = &self.search_client(chat_id, user_id).await?;

        Ok(row.get("state"))
    }

    pub async fn search_client(&self, chat_id: &i64, user_id: u64) -> Result<Row, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let row = connection
            .query_one(SEARCH_CLIENT, &[chat_id, &bytes])
            .await?;

        Ok(row)
    }
    pub async fn insert_client(
        &self,
        chat_id: &i64,
        user_id: u64,
        user: String,
        keypair: &PKey<Private>,
    ) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();
        let user_encrypted = Self::encrypt_string(user, keypair)?;

        let n = connection
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

    pub async fn modify_city(
        &self,
        chat_id: &i64,
        user_id: u64,
        new_city: String,
    ) -> Result<u64, BotDbError> {
        let connection = self.pool.get().await?;

        let bytes = user_id.to_le_bytes().to_vec();

        let n = connection
            .execute(MODIFY_CITY, &[&new_city, chat_id, &bytes])
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

    pub async fn get_city_by_pattern(&self, city: &str) -> Result<Vec<Row>, BotDbError> {
        let connection = self.pool.get().await?;

        let st = format!("%{}%", city.to_uppercase());

        let vec = connection.query(GET_CITY_BY_PATTERN, &[&st]).await?;
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
    use bb8_postgres::tokio_postgres::Row;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;

    #[tokio::test]
    async fn test_modify_state() {
        // Pick a random user of the DB
        let db_controller = DbController::new().await.unwrap();
        let connection = db_controller.pool.get().await.unwrap();

        let binary_file = std::fs::read("./resources/key.pem").unwrap();
        let keypair = Rsa::private_key_from_pem(&binary_file).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        let n = db_controller
            .insert_client(&111111, 1111111, String::from("@ItzPXP9"), &keypair)
            .await
            .unwrap();

        assert_eq!(n, 1_u64);

        let row: &Row = &connection
            .query_one("SELECT * FROM chat LIMIT 1", &[])
            .await
            .unwrap();

        let chat_id: i64 = row.get("id");

        let bytes: &[u8] = row.get("user_id");
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        let user_id = as_u64_le(&arr);

        let user: String = DbController::decrypt_string(row.get("user"), &keypair).unwrap();
        assert_eq!(user, String::from("@ItzPXP9"));
        // testing modify state

        let n = db_controller
            .modify_state(&chat_id, user_id, ClientState::Pattern)
            .await
            .unwrap();

        assert_eq!(n, 1_u64);

        // testing get state
        let actual_state = db_controller
            .get_client_state(&chat_id, user_id)
            .await
            .unwrap();

        assert_eq!(actual_state, ClientState::Pattern);

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
