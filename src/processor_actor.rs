use tokio::sync::{oneshot, mpsc};
use crate::process_data::{ProcessedData, process_data};
use rust_decimal::Decimal;
use crate::download_data::YQuote;
use tokio::time::Duration;

// ----------------------------------------------------------------------------- msg

enum ActorMessage {
    ProcessData {
        quotes: Vec<YQuote>,
        ticker: String,
    },
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
            ActorMessage::ProcessData { quotes, ticker} => {
                println!("processing");
                process_data(quotes, ticker);
            }
            _ => unreachable!()
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
        let (sender, receiver) = mpsc::channel(8); //todo how bounded do I want it?
        //first call new()
        let mut actor = ProcessorActor::new(receiver);
        //then call run()
        tokio::spawn(async move { actor.run().await });
        Self { sender }
    }
    pub async fn process_data(&self, quotes: Vec<YQuote>, ticker: String) {
        let _ = self.sender.send(ActorMessage::ProcessData { quotes, ticker }).await;

        //todo
        //send method on a bounded channel does not return immediately
        //send method on a one-off channel DOES return immediately

        //if you don't want to hear back and
        // the channel can get full - you need to make this an async function
        // otherwise use try_send and handle sending failures by killing the actor
    }
}

