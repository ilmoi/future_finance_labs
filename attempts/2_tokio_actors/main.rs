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

        // fetch data
        let quotes = downloader_handle.download_data(ticker.into(), from, to).await;

        // process data
        let processor_handle = ProcessorActorHandle::new();
        processor_handle.process_data(quotes, ticker.into()).await;
    }
}

