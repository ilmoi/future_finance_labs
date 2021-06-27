use crate::download_data::fetch_stonks_data;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use yahoo_finance_api::YahooConnector;

pub struct DownloadMsg {
    pub ticker: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

pub struct DActor {
    receiver: mpsc::Receiver<DownloadMsg>,
}

impl DActor {
    pub fn new(receiver: mpsc::Receiver<DownloadMsg>) -> Self {
        Self { receiver }
    }
    pub async fn run(&mut self) {
        println!("downloader actor spawned and running!");
        while let Some(msg) = self.receiver.recv().await {
            self.handle_msg(msg).await; //don't forget the await here or you'll get a panic ("actor task has been killed and failed to respond")
        }
    }
    pub async fn handle_msg(&self, msg: DownloadMsg) {
        println!("handling");
        let provider = YahooConnector::new();
        let data = fetch_stonks_data(&provider, msg.ticker.clone(), msg.from, msg.to)
            .await
            .unwrap();
        println!("done handling");
    }
}

// pub async fn run_my_actor(mut actor: DActor) {
//     while let Ok(msg) = actor.receiver.recv().await {
//         actor.handle_msg(msg).await;
//     }
// }
