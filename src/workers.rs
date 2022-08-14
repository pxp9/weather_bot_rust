use crate::DATABASE_URL;
use fang::asynk::async_queue::AsyncQueue;
use fang::asynk::async_worker_pool::AsyncWorkerPool;
use fang::NoTls;

pub static NUMBER_OF_WORKERS: u32 = 5;

pub async fn start_workers() {
    let mut queue: AsyncQueue<NoTls> = AsyncQueue::builder()
        .uri(DATABASE_URL.clone())
        .max_pool_size(NUMBER_OF_WORKERS)
        .duplicated_tasks(true)
        .build();

    queue.connect(NoTls).await.unwrap();

    let mut pool: AsyncWorkerPool<AsyncQueue<NoTls>> = AsyncWorkerPool::builder()
        .number_of_workers(NUMBER_OF_WORKERS)
        .queue(queue.clone())
        .build();

    pool.start().await;
}
