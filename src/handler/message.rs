use http_mitm::{async_trait::async_trait, tungstenite::Message, MessageContext, MessageHandler};

#[derive(Clone)]
pub struct NoopMessageHandler {}

#[async_trait]
impl MessageHandler for NoopMessageHandler {
    async fn handle_message(&mut self, _ctx: &MessageContext, msg: Message) -> Option<Message> {
        Some(msg)
    }
}
