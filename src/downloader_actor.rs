use tokio::sync::{oneshot, mpsc};
use rust_decimal::Decimal;
use crate::download_data::{YQuote, fetch_stonks_data};
use tokio::time::Duration;
use chrono::{Utc, DateTime};
use yahoo_finance_api::YahooConnector;
use std::sync::Arc;

// ----------------------------------------------------------------------------- msg

enum ActorMessage {
    DownloadData {
        ticker: String,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        respond_to: oneshot::Sender<Vec<YQuote>>
    },
}

// ----------------------------------------------------------------------------- actors

struct DownloaderActor {
    receiver: mpsc::Receiver<ActorMessage>,
    provider: YahooConnector,
}

impl DownloaderActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>, provider: YahooConnector) -> Self {
        Self { receiver, provider }
    }
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await; //don't forget the await here or you'll get a panic ("actor task has been killed and failed to respond")
        }
    }
    //this fn has to be async to be able to call the async fetch_stonks_data function
    async fn handle_message(&mut self, msg: ActorMessage) {
        if let ActorMessage::DownloadData { ticker, from, to, respond_to} = msg {
            println!("downloading");
            let data = fetch_stonks_data(&self.provider, ticker, from, to).await.unwrap();
            respond_to.send(data);
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
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8); //todo how bounded do I want it?
        //first call new()
        let provider = yahoo_finance_api::YahooConnector::new();
        let mut actor = DownloaderActor::new(receiver, provider);
        //then call run()
        tokio::spawn(async move { actor.run().await });
        Self { sender }
    }
    pub async fn download_data(&self, ticker: String, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<YQuote> {
        let (respond_to, listen_on) = oneshot::channel();
        let _ = self.sender.send(ActorMessage::DownloadData { ticker, from, to, respond_to }).await;
        listen_on.await.expect("actor task has been killed and failed to respond")
    }
}

