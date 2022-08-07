use super::client::ApiClient;
use super::process_update_task::ProcessUpdateTask;
use fang::asynk::async_queue::AsyncQueue;
use fang::asynk::async_queue::AsyncQueueable;
use fang::NoTls;
use std::env;

pub struct Handler {
    client: ApiClient,
    // TODO: use in memory queue
    queue: AsyncQueue<NoTls>,
}

impl Handler {
    pub fn new() -> Self {
        let client = ApiClient::new();
        let queue = Self::init_queue();

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
        }
    }

    fn init_queue() -> AsyncQueue<NoTls> {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");

        AsyncQueue::builder()
            .uri(database_url)
            .max_pool_size(1_u32)
            .duplicated_tasks(true)
            .build()
    }
}
