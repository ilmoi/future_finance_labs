use std::io;

use chrono::{DateTime, TimeZone, Utc};
use clap::Clap;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use future_finance_labs::download_data::fetch_stonks_data;
use future_finance_labs::downloader_actor::DownloaderActorHandle;
use future_finance_labs::process_data::{extract_adjclose, min_and_max, n_window_sma, price_diff};
use future_finance_labs::processor_actor::ProcessorActorHandle;
use tokio::net::TcpListener;
use yahoo_finance_api::YahooConnector;

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

#[tokio::main]
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

    for ticker in opts.tickers.split(",").collect::<Vec<&str>>() {
        // todo because there is no publish / subscribe we're forced to create a new actor on each iteration of the loop
        //  that's because we need to send each new task to a new actor
        //  this sort of destroys the purpose of having actors.. we could have just used an event loop
        //  in short - this type of actor is the wrong abstraction here - we need publish/subscribe type actors which tokio doesn't have
        let processor_handle = ProcessorActorHandle::new();
        let downloader_handle = DownloaderActorHandle::new(processor_handle);

        downloader_handle
            .download_data(ticker.into(), from, to)
            .await;
    }

    //todo what's worse in this type of design we have to keep spinning main in a loop, otherwise it exits early and async tasks never complete
    // that's coz none of the actors send return msgs, so they return immediately and main loop has nothing else to work on, so it exits
    loop {}
}
