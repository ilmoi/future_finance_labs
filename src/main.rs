use std::io;

use chrono::{DateTime, TimeZone, Utc};
use clap::Clap;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use async_std::prelude::*;
use async_std::stream;
use xactor::{message, Actor, Broker, Context, Handler, Result, Service, Supervisor};

use future_finance_labs::download_data::fetch_stonks_data;
use future_finance_labs::process_data::{
    extract_adjclose, min_and_max, n_window_sma, price_diff, process_data, Data,
};
use std::time::Duration;

//simpler but defo lacking functionality vs normal builder pattern
//can't pass in Utc::now() as default value
//can't pass in as const / static
//had to the whole dance with default_value(x) and then correct as unwrap_or()
#[derive(Clap)]
#[clap(
    version = "0.1",
    author = "ilmoi",
    about = "future finance labs cli app"
)]
struct Opts {
    //Stonk tickers - lower or upper-case
    #[clap(long, default_value = "AAPL,MSFT,UBER,GOOG")]
    tickers: String,
    ///Date in yyyy-mm-dd format. Default = 1 month ago.
    #[clap(short, long, default_value = "x")]
    from: String,
    ///Date in yyyy-mm-dd format. Default = now.
    #[clap(short, long, default_value = "x")]
    to: String,
}

// ----------------------------------------------------------------------------- msg

#[message]
#[derive(Clone, Debug)]
struct DownloadMsg {
    ticker: String,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

#[message]
#[derive(Clone, Debug)]
struct ProcessMsg {
    data: Data,
    ticker: String,
}

// ----------------------------------------------------------------------------- actor

#[derive(Default)]
struct DownloadActor;

#[derive(Default)]
struct ProcessActor;

#[async_trait::async_trait]
impl Actor for DownloadActor {
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        ctx.subscribe::<DownloadMsg>().await;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Actor for ProcessActor {
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        ctx.subscribe::<ProcessMsg>().await;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Handler<DownloadMsg> for DownloadActor {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: DownloadMsg) {
        let data = fetch_stonks_data(msg.ticker.clone(), msg.from, msg.to)
            .await
            .unwrap();
        //once Download Actor finishes its work, it publishes a msg to the next q, which is the processing q, to be picked up by processing actors
        let _ = Broker::from_registry().await.unwrap().publish(ProcessMsg {
            data,
            ticker: msg.ticker,
        });
    }
}

#[async_trait::async_trait]
impl Handler<ProcessMsg> for ProcessActor {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: ProcessMsg) {
        process_data(msg.data, msg.ticker);
    }
}

// ----------------------------------------------------------------------------- main

#[xactor::main]
async fn main() {
    let opts = Opts::parse();
    let from: DateTime<Utc> = opts
        .from
        .parse()
        .unwrap_or(Utc::now() - chrono::Duration::days(60));
    let to: DateTime<Utc> = opts.to.parse().unwrap_or(Utc::now());

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(&[
        "period start",
        "symbol",
        "price",
        "change %",
        "min",
        "max",
        "30d avg",
    ])
    .unwrap();
    wtr.flush().unwrap();

    // weird: if you don't collect addresses, the program stalls
    // todo weird 2: if you start more than one actor - ALL of them get msgs
    //  if this can't be fixed this solution is actually WORSE than my solution with tokio actors...
    //  https://github.com/sunli829/xactor/issues/45
    let _daddr = DownloadActor::start_default().await.unwrap();
    // let _daddr2 = DownloadActor::start_default().await.unwrap();
    let _paddr = ProcessActor::start_default().await.unwrap();
    // let _paddr2 = ProcessActor::start_default().await.unwrap();

    // their way - doesn't work for me, I don't see any msgs processed
    // let downloader = Supervisor::start(|| DownloadActor);
    // let processor = Supervisor::start(|| ProcessActor);
    // let _ = downloader.join(processor).await;

    // todo same story with the loop - if main isn't looping, actors won't have time to act
    let mut interval = stream::interval(Duration::from_secs(10));
    while interval.next().await.is_some() {
        for ticker in opts.tickers.split(",").collect::<Vec<&str>>() {
            // prep msg
            let msg = DownloadMsg {
                ticker: ticker.into(),
                from,
                to,
            };
            // send it
            let _ = Broker::from_registry().await.unwrap().publish(msg);
        }
    }
}
