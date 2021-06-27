use crate::download_data::YQuote;
use crate::process_data::{process_data, ProcessedData};
use rust_decimal::Decimal;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Duration;

// ----------------------------------------------------------------------------- msg

enum ActorMessage {
    ProcessData { quotes: Vec<YQuote>, ticker: String },
}

// ----------------------------------------------------------------------------- actors

struct ProcessorActor {
    receiver: mpsc::Receiver<ActorMessage>,
}

impl ProcessorActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        Self { receiver }
    }
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }
    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::ProcessData { quotes, ticker } => {
                process_data(quotes, ticker);
            }
            _ => unreachable!(),
        }
    }
}

// ----------------------------------------------------------------------------- handle

#[derive(Clone)]
pub struct ProcessorActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

/// Note this is the only public interface to the actor system that we're exposing
impl ProcessorActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(10);
        //first call new()
        let mut actor = ProcessorActor::new(receiver);
        //then call run()
        tokio::spawn(async move { actor.run().await });
        Self { sender }
    }
    pub async fn process_data(&self, quotes: Vec<YQuote>, ticker: String) {
        let _ = self
            .sender
            .send(ActorMessage::ProcessData { quotes, ticker })
            .await;
    }
}
