use crate::download_data::{fetch_stonks_data, YQuote};
use crate::process_data::Data;
use crate::processor_actor::ProcessorActorHandle;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::time::Duration;
use yahoo_finance_api::YahooConnector;

// ----------------------------------------------------------------------------- msg

enum ActorMessage {
    DownloadData {
        ticker: String,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        processor_handle: ProcessorActorHandle,
    },
}

// ----------------------------------------------------------------------------- actors

struct DownloaderActor {
    receiver: async_channel::Receiver<ActorMessage>,
    provider: YahooConnector,
}

impl DownloaderActor {
    fn new(receiver: async_channel::Receiver<ActorMessage>, provider: YahooConnector) -> Self {
        Self { receiver, provider }
    }
    async fn run(&mut self) {
        println!("downloader actor spawned and running!");
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg).await; //don't forget the await here or you'll get a panic ("actor task has been killed and failed to respond")
        }
    }
    //this fn has to be async to be able to call the async fetch_stonks_data function
    async fn handle_message(&mut self, msg: ActorMessage) {
        if let ActorMessage::DownloadData {
            ticker,
            from,
            to,
            processor_handle,
        } = msg
        {
            println!(
                "starting to handle download msg..., {}, {}, {}",
                &ticker, from, to
            );
            //todo clone may not be ideal
            let data = fetch_stonks_data(&self.provider, ticker.clone(), from, to)
                .await
                .unwrap_or_else(|_| {
                    println!("wtf");
                    vec![]
                });

            println!("ending to handle download msg...");
            // processor_handle.process_data(data, ticker.clone()).await;
        }
    }
}

// ----------------------------------------------------------------------------- handle

#[derive(Clone)]
pub struct DownloaderActorHandle {
    sender: async_channel::Sender<ActorMessage>,
}

/// Note this is the only public interface to the actor system that we're exposing
impl DownloaderActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::bounded(10);

        // todo potentially below in a loop to have more than one actor
        //first call new()
        let provider = yahoo_finance_api::YahooConnector::new();
        let mut actor = DownloaderActor::new(receiver, provider);
        //then call run()

        // todo OK AT LEAST I KNOW THIS IS THE PROBLEM
        //  could it be that it just doesn't play nicely with tokio and I should use flume?
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }
    pub async fn download_data(
        &self,
        ticker: String,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        processor_handle: ProcessorActorHandle,
    ) {
        self.sender
            .send(ActorMessage::DownloadData {
                ticker,
                from,
                to,
                processor_handle,
            })
            .await;
    }
}
