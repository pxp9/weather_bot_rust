use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::asynk::AsyncError as Error;
use fang::serde::Deserialize;
use fang::serde::Serialize;
use fang::typetag;
use fang::AsyncRunnable;
use frankenstein::Update;

const TASK_TYPE: &str = "update";

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "fang::serde")]
pub struct ProcessUpdateTask {
    update: Update,
}

impl ProcessUpdateTask {
    pub fn new(update: Update) -> Self {
        Self { update }
    }

    pub fn process(&self) {
        // TODO: process users messages here
        log::info!("Received a message {:?}", self.update);
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for ProcessUpdateTask {
    async fn run(&self, _queueable: &mut dyn AsyncQueueable) -> Result<(), Error> {
        self.process();

        Ok(())
    }

    fn task_type(&self) -> String {
        TASK_TYPE.to_string()
    }
}
