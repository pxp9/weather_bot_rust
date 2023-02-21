use crate::command::process_update_task::TASK_TYPE;
use crate::deliver::DeliverChecker;
use crate::deliver::SCHEDULED_TASK_TYPE;
use crate::DATABASE_URL;
use fang::asynk::async_queue::AsyncQueue;
use fang::asynk::async_worker_pool::AsyncWorkerPool;
use fang::AsyncQueueable;
use fang::FangError;
use fang::NoTls;
use fang::SleepParams;
use std::time::Duration;

pub static NUMBER_OF_WORKERS: u32 = 5;

pub async fn start_workers() -> Result<(), FangError> {
    let mut queue: AsyncQueue<NoTls> = AsyncQueue::builder()
        .uri(DATABASE_URL.clone())
        .max_pool_size(NUMBER_OF_WORKERS)
        .build();

    queue.connect(NoTls).await.unwrap();

    let params = SleepParams {
        sleep_period: Duration::from_millis(250),
        max_sleep_period: Duration::from_millis(1500),
        min_sleep_period: Duration::from_secs(0),
        sleep_step: Duration::from_millis(250),
    };

    let mut pool: AsyncWorkerPool<AsyncQueue<NoTls>> = AsyncWorkerPool::builder()
        .number_of_workers(NUMBER_OF_WORKERS)
        .sleep_params(params.clone())
        .queue(queue.clone())
        .task_type(TASK_TYPE)
        .build();

    let mut pool_scheduled: AsyncWorkerPool<AsyncQueue<NoTls>> = AsyncWorkerPool::builder()
        .number_of_workers(NUMBER_OF_WORKERS)
        .sleep_params(params)
        .queue(queue.clone())
        .task_type(SCHEDULED_TASK_TYPE)
        .build();

    pool.start().await;
    pool_scheduled.start().await;

    queue.schedule_task(&DeliverChecker {}).await?;
    Ok(())
}
