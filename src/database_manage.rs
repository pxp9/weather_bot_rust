use openssl::encrypt::{Decrypter, Encrypter};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Padding;
use tokio_postgres::{Error, Row, Transaction};
// Encrypt a String into bytea
async fn encrypt_string(some_string: String, keypair: &PKey<Private>) -> Vec<u8> {
    let mut encrypter = Encrypter::new(keypair).unwrap();
    encrypter.set_rsa_padding(Padding::PKCS1).unwrap();
    let st_bytes = some_string.as_bytes();
    let len: usize = encrypter.encrypt_len(&st_bytes).unwrap();
    let mut encrypted = vec![0; len];
    let encrypted_len = encrypter.encrypt(st_bytes, &mut encrypted).unwrap();
    encrypted.truncate(encrypted_len);
    encrypted
}
// Decrypt a BYTEA into a String
async fn decrypt_string(encrypted: &[u8], keypair: &PKey<Private>) -> String {
    let mut decrypter = Decrypter::new(&keypair).unwrap();
    decrypter.set_rsa_padding(Padding::PKCS1).unwrap();
    let buffer_len = decrypter.decrypt_len(encrypted).unwrap();
    let mut decrypted = vec![0; buffer_len];
    let decrypted_len = decrypter.decrypt(encrypted, &mut decrypted).unwrap();
    decrypted.truncate(decrypted_len);
    String::from_utf8(decrypted).unwrap()
}
pub async fn is_in_db(db_transaction: &mut Transaction<'_>, chat_id: &i64) -> Result<bool, Error> {
    Ok(db_transaction
        .execute(
            "SELECT \"user\" , state , city FROM chat WHERE id = $1",
            &[chat_id],
        )
        .await?
        == 1)
}
pub async fn search_city(
    db_transaction: &mut Transaction<'_>,
    n: &String,
    c: &String,
    s: &String,
) -> Result<(f64, f64, String, String, String), ()> {
    let vec: Vec<Row> = db_transaction
        .query(
            "SELECT name , country , state , lon , lat FROM cities WHERE 
        UPPER(name) = UPPER($1) AND UPPER(country) = UPPER($2)
            AND UPPER(state) = UPPER($3)",
            &[n, c, s],
        )
        .await
        .unwrap();
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
pub async fn get_client_state(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
) -> Result<String, Error> {
    let row: &Row = &search_client(db_transaction, chat_id).await?[0];
    Ok(row.get("state"))
}
pub async fn search_client(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
) -> Result<Vec<Row>, Error> {
    let search = db_transaction
        .prepare("SELECT \"user\" , state , city FROM chat WHERE id = $1")
        .await?;
    Ok(db_transaction.query(&search, &[chat_id]).await?)
}
pub async fn insert_client(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    user: String,
    default_city: String,
    keypair: &PKey<Private>,
) -> Result<u64, Error> {
    let insert = db_transaction
        .prepare("INSERT INTO chat (id , \"user\" , state , city ) VALUES ($1 , $2 , $3 , $4)")
        .await?;
    let user_encrypted = encrypt_string(user, keypair).await;
    Ok(db_transaction
        .execute(&insert, &[chat_id, &user_encrypted, &"IN", &default_city])
        .await?)
}
pub async fn delete_client(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
) -> Result<u64, Error> {
    let delete = db_transaction
        .prepare("DELETE FROM chat WHERE id = $1")
        .await?;
    Ok(db_transaction.execute(&delete, &[chat_id]).await?)
}
pub async fn modify_state(
    db_transaction: &mut Transaction<'_>,
    chat_id: &i64,
    new_state: String,
) -> Result<u64, Error> {
    let modify_state = db_transaction
        .prepare("UPDATE chat SET state = $1 WHERE id = $2")
        .await?;
    Ok(db_transaction
        .execute(&modify_state, &[&new_state, chat_id])
        .await?)
}
