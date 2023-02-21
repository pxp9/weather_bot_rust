use super::client::ApiClient;
use crate::command::ProcessUpdateTask;
use crate::DATABASE_URL;
use fang::asynk::async_queue::AsyncQueue;
use fang::asynk::async_queue::AsyncQueueable;
use fang::NoTls;
use std::time::Duration;
use tokio::time::sleep;

pub struct Handler {
    client: ApiClient,
    queue: AsyncQueue<NoTls>,
}

impl Handler {
    pub async fn new() -> Self {
        let client = ApiClient::api_client().await.clone();
        let queue = Self::init_queue().await;

        Self { client, queue }
    }

    pub async fn start(&mut self) {
        loop {
            while let Some(update) = self.client.next_update().await {
                let task = ProcessUpdateTask::new(update);

                if let Err(err) = self.queue.insert_task(&task).await {
                    log::error!(
                        "Failed to enqueue ProcessUpdateTask task {:?}, error {:?}",
                        task,
                        err
                    );
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    }

    async fn init_queue() -> AsyncQueue<NoTls> {
        let mut queue: AsyncQueue<NoTls> = AsyncQueue::builder()
            .uri(DATABASE_URL.clone())
            .max_pool_size(1_u32)
            .build();

        queue.connect(NoTls).await.unwrap();

        queue
    }
}
