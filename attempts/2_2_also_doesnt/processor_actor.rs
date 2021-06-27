use crate::download_data::YQuote;
use crate::process_data::{process_data, Data, ProcessedData};
use rust_decimal::Decimal;
use tokio::time::Duration;

// ----------------------------------------------------------------------------- msg

enum ActorMessage {
    ProcessData { quotes: Data, ticker: String },
}

// ----------------------------------------------------------------------------- actors

struct ProcessorActor {
    receiver: async_channel::Receiver<ActorMessage>,
}

impl ProcessorActor {
    fn new(receiver: async_channel::Receiver<ActorMessage>) -> Self {
        Self { receiver }
    }
    async fn run(&mut self) {
        println!("processor actor spawned and running!");
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }
    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::ProcessData { quotes, ticker } => {
                println!("processing");
                process_data(quotes, ticker);
            }
            _ => unreachable!(),
        }
    }
}

// ----------------------------------------------------------------------------- handle

#[derive(Clone)]
pub struct ProcessorActorHandle {
    sender: async_channel::Sender<ActorMessage>,
}

/// Note this is the only public interface to the actor system that we're exposing
impl ProcessorActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::bounded(10);

        //todo potentially in a loop
        //first call new()
        let mut actor = ProcessorActor::new(receiver);
        //then call run()
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }
    pub async fn process_data(&self, quotes: Data, ticker: String) {
        let _ = self
            .sender
            .send(ActorMessage::ProcessData { quotes, ticker })
            .await;
    }
}
