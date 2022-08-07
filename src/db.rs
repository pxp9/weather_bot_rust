use openssl::encrypt::{Decrypter, Encrypter};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Padding;
use std::include_str;
use tokio_postgres::{Error, Row, Transaction};

const _CREATE_DB: &str = include_str!("queries/create_db.sql");
const _DROP_DB: &str = include_str!("queries/drop_db.sql");
const _CREATE_TABLE_CHAT: &str = include_str!("queries/create_table_chat.sql");
const _CREATE_TABLE_CITIES: &str = include_str!("queries/create_table_cities.sql");
const DELETE_CLIENT: &str = include_str!("queries/delete_client.sql");
const GET_CITY_BY_PATTERN: &str = include_str!("queries/get_city_by_pattern.sql");
const INSERT_CLIENT: &str = include_str!("queries/insert_client.sql");
const IS_IN_DB: &str = include_str!("queries/is_in_db.sql");
const MODIFY_CITY: &str = include_str!("queries/modify_city.sql");
const MODIFY_CONTEXT: &str = include_str!("queries/modify_context.sql");
const MODIFY_PATTERN_SEARCH: &str = include_str!("queries/modify_pattern_search.sql");
const MODIFY_SELECTED: &str = include_str!("queries/modify_selected.sql");
const MODIFY_STATE: &str = include_str!("queries/modify_state.sql");
const SEARCH_CITY: &str = include_str!("queries/search_city.sql");
const SEARCH_CLIENT: &str = include_str!("queries/search_client.sql");

async fn _migrate(db_transaction: &mut Transaction<'_>) -> Result<(), Error> {
    db_transaction.execute(_CREATE_TABLE_CHAT, &[]).await?;
    db_transaction.execute(_CREATE_TABLE_CITIES, &[]).await?;
    Ok(())
}
async fn _setup_db(mut db_transaction: Transaction<'_>) -> Result<(), Error> {
    db_transaction.execute(_CREATE_DB, &[]).await?;
    _migrate(&mut db_transaction).await?;
    db_transaction.commit().await
}
async fn _rollback(db_transaction: Transaction<'_>) -> Result<(), Error> {
    db_transaction.rollback().await
}
async fn _drop_db(db_transaction: Transaction<'_>) -> Result<(), Error> {
    db_transaction.execute(_DROP_DB, &[]).await?;
    db_transaction.commit().await
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
async fn _decrypt_string(encrypted: &[u8], keypair: &PKey<Private>) -> String {
    let mut decrypter = Decrypter::new(keypair).unwrap();
    decrypter.set_rsa_padding(Padding::PKCS1).unwrap();
    let buffer_len = decrypter.decrypt_len(encrypted).unwrap();
    let mut decrypted = vec![0; buffer_len];
    let decrypted_len = decrypter.decrypt(encrypted, &mut decrypted).unwrap();
    decrypted.truncate(decrypted_len);
    String::from_utf8(decrypted).unwrap()
}
pub async fn is_in_db(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<bool, Error> {
    let bytes = user_id.to_le_bytes().to_vec();
    Ok(db_transaction.execute(IS_IN_DB, &[chat_id, &bytes]).await? == 1)
}
pub async fn search_city(
    db_transaction: &mut Transaction<'_>,
    n: &String,
    c: &String,
    s: &String,
) -> Result<(f64, f64, String, String, String), ()> {
    let vec: Vec<Row> = db_transaction.query(SEARCH_CITY, &[n, c, s]).await.unwrap();
    if vec.len() == 1 {
        Ok((
            vec[0].get("lon"),
            vec[0].get("lat"),
            vec[0].get("name"),
            vec[0].get("country"),
            vec[0].get("state"),
        ))
    } else {
        Err(())
    }
}
pub async fn get_client_selected(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<String, Error> {
    let row: &Row = &search_client(db_transaction, chat_id, user_id).await?;
    Ok(row.get("selected"))
}

pub async fn get_client_city(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<String, Error> {
    let row: &Row = &search_client(db_transaction, chat_id, user_id).await?;
    row.try_get("city")
}

pub async fn get_client_context(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<String, Error> {
    let row: &Row = &search_client(db_transaction, chat_id, user_id).await?;
    Ok(row.get("context"))
}
pub async fn get_client_state(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<String, Error> {
    let row: &Row = &search_client(db_transaction, chat_id, user_id).await?;
    Ok(row.get("state"))
}

pub async fn get_client_pattern_search(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<bool, Error> {
    let row: &Row = &search_client(db_transaction, chat_id, user_id).await?;
    row.try_get("pattern_search")
}
pub async fn search_client(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<Row, Error> {
    let bytes = user_id.to_le_bytes().to_vec();
    db_transaction
        .query_one(SEARCH_CLIENT, &[chat_id, &bytes])
        .await
}
pub async fn insert_client(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
    user: String,
    keypair: &PKey<Private>,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();
    let user_encrypted = encrypt_string(user, keypair).await;

    db_transaction
        .execute(
            INSERT_CLIENT,
            &[chat_id, &user_encrypted, &"Initial", &"Initial", &bytes],
        )
        .await
}
pub async fn delete_client(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();

    db_transaction
        .execute(DELETE_CLIENT, &[chat_id, &bytes])
        .await
}
pub async fn modify_state(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
    new_state: String,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();

    db_transaction
        .execute(MODIFY_STATE, &[&new_state, chat_id, &bytes])
        .await
}

pub async fn modify_context(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
    new_context: String,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();

    db_transaction
        .execute(MODIFY_CONTEXT, &[&new_context, chat_id, &bytes])
        .await
}

pub async fn modify_city(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
    new_city: String,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();

    db_transaction
        .execute(MODIFY_CITY, &[&new_city, chat_id, &bytes])
        .await
}
pub async fn modify_selected(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
    new_selected: String,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();

    db_transaction
        .execute(MODIFY_SELECTED, &[&new_selected, chat_id, &bytes])
        .await
}

pub async fn modify_pattern_search(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user_id: u64,
    search_mode: bool,
) -> Result<u64, Error> {
    let bytes = user_id.to_le_bytes().to_vec();

    db_transaction
        .execute(MODIFY_PATTERN_SEARCH, &[&search_mode, chat_id, &bytes])
        .await
}
pub async fn get_city_by_pattern(
    db_transaction: &mut Transaction<'_>,
    city: &str,
) -> Result<Vec<Row>, Error> {
    let st = format!("%{}%", city.to_uppercase());

    db_transaction.query(GET_CITY_BY_PATTERN, &[&st]).await
}
pub async fn get_city_row(
    db_transaction: &mut Transaction<'_>,
    city: &str,
    n: usize,
) -> Result<(String, String, String), ()> {
    let vec: Vec<Row> = get_city_by_pattern(db_transaction, city).await.unwrap();
    if n > vec.len() {
        return Err(());
    }
    Ok((
        vec[n - 1].get("name"),
        vec[n - 1].get("country"),
        vec[n - 1].get("state"),
    ))
}

#[cfg(test)]
mod db_test {
    use crate::db::*;
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
        // testing modify state
        let n = modify_state(&mut transaction, &chat_id, String::from("AskingCity"))
            .await
            .unwrap();
        assert_eq!(n, 1 as u64);
        transaction.commit().await.unwrap();
        let mut transaction = client.transaction().await.unwrap();

        // testing get state
        let actual_state = get_client_state(&mut transaction, &chat_id).await.unwrap();
        assert_eq!(actual_state, String::from("AskingCity"));

        let n = modify_state(&mut transaction, &chat_id, String::from("Initial"))
            .await
            .unwrap();
        assert_eq!(n, 1 as u64);
        transaction.commit().await.unwrap();

        let mut transaction = client.transaction().await.unwrap();
        let actual_state = get_client_state(&mut transaction, &chat_id).await.unwrap();
        assert_eq!(actual_state, String::from("Initial"));
        transaction.commit().await.unwrap();
    }
}
