use crate::download_data::{fetch_stonks_data, YQuote};
use crate::processor_actor::ProcessorActorHandle;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Duration;
use yahoo_finance_api::YahooConnector;

// ----------------------------------------------------------------------------- msg

enum ActorMessage {
    DownloadData {
        ticker: String,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
}

// ----------------------------------------------------------------------------- actors

struct DownloaderActor {
    receiver: mpsc::Receiver<ActorMessage>,
    processor_handle: ProcessorActorHandle,
}

impl DownloaderActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>, processor_handle: ProcessorActorHandle) -> Self {
        Self {
            receiver,
            processor_handle,
        }
    }
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }
    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::DownloadData { ticker, from, to } => {
                let data = fetch_stonks_data(ticker.clone(), from, to).await.unwrap();
                self.processor_handle.process_data(data, ticker).await;
            }
            _ => unreachable!(),
        }
    }
}

// ----------------------------------------------------------------------------- handle

#[derive(Clone)]
pub struct DownloaderActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

/// Note this is the only public interface to the actor system that we're exposing
impl DownloaderActorHandle {
    pub fn new(processor_handle: ProcessorActorHandle) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        //first call new()
        let mut actor = DownloaderActor::new(receiver, processor_handle);
        //then call run()
        tokio::spawn(async move { actor.run().await });
        Self { sender }
    }
    pub async fn download_data(&self, ticker: String, from: DateTime<Utc>, to: DateTime<Utc>) {
        let _ = self
            .sender
            .send(ActorMessage::DownloadData { ticker, from, to })
            .await;
    }
}
