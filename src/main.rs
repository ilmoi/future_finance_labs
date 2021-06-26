use std::io;

use chrono::{DateTime, TimeZone, Utc};
use clap::Clap;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

use future_finance_labs::process_data::{extract_adjclose, min_and_max, n_window_sma, price_diff};
use future_finance_labs::download_data::fetch_stonks_data;
use future_finance_labs::processor_actor::ProcessorActorHandle;
use future_finance_labs::downloader_actor::DownloaderActorHandle;

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
    #[clap(short, long, default_value="x")]
    from: String,
    ///Date in yyyy-mm-dd format. Default = now.
    #[clap(short, long, default_value="x")]
    to: String,
}

#[tokio::main]
pub async fn main() {
    let opts = Opts::parse();
    let from:DateTime<Utc> = opts.from.parse().unwrap_or(Utc::now() - chrono::Duration::days(60));
    let to:DateTime<Utc> = opts.to.parse().unwrap_or(Utc::now());

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(&["period start", "symbol", "price", "change %", "min", "max", "30d avg"]).unwrap();
    wtr.flush().unwrap();

    let downloader_handle = DownloaderActorHandle::new();

    for ticker in opts.tickers.split(",").collect::<Vec<&str>>() {

        // todo q1: how are actors better than a simple event loop?
        //  - actors shine when you have a single resource that you can't multiply and want to share across many tasks
        //  - eg if we only had 1 connection to yahoo api, we'd send all requests to that one actor who'd process them and return the results to each task separately
        //  - see an example here - https://tokio.rs/tokio/tutorial/channels = single conn, can't copy, can't do mutex, can't do tokio mutex - so the only way is to put inside an actor

        // todo q2: does the architecture of downloader-actor / processor-actor make sense?
        //  - not really
        //  - downloader blocks processor, so they might as well be part of one and the same async task
        //  - in other words, a single tokio::spawn for the pair of download+process data should be enough

        // todo so to sum up - might as well have gone with a simple async event loop, with both tasks inside of one iter of it
        //  UNLESS I implemented a full q system - where all downloaders dump data in, all processors pick data up - which I think is what those guys did
        //  tokio doesn't do mpmc - https://github.com/tokio-rs/tokio/discussions/3891

        // fetch data
        let quotes = downloader_handle.download_data(ticker.into(), from, to).await;

        // process data
        let processor_handle = ProcessorActorHandle::new();
        processor_handle.process_data(quotes, ticker.into()).await;
    }
}

