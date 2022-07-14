#[cfg(test)]
mod db {
    use crate::database_manage::*;
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
