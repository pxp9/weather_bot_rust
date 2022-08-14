use async_trait::async_trait;
use frankenstein::Message;
use

#[async_trait]
pub trait Command {
    async fn process() {
    }
}
