use std::cmp::Ordering;
use std::error::Error;
use std::io;
use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, TimeZone, Utc};
use clap::{App, Arg, Clap};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use yahoo_finance_api::Quote;

#[derive(Debug, PartialEq, PartialOrd)]
struct YQuote {
    timestamp: u64,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    volume: u64,
    close: Decimal,
    adjclose: Decimal,
}

impl From<Quote> for YQuote {
    fn from(q: Quote) -> Self {
        Self {
            timestamp: q.timestamp,
            open: Decimal::from_str(&q.open.to_string()).unwrap(), //todo could be done better - chop off X digits and go via int + currently using unwrap instead of proper deserialization
            high: Decimal::from_str(&q.high.to_string()).unwrap(),
            low: Decimal::from_str(&q.low.to_string()).unwrap(),
            volume: q.volume,
            close: Decimal::from_str(&q.close.to_string()).unwrap(),
            adjclose: Decimal::from_str(&q.adjclose.to_string()).unwrap(),
        }
    }
}

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

fn main() {
    let opts = Opts::parse();
    let from:DateTime<Utc> = opts.from.parse().unwrap_or(Utc::now() - chrono::Duration::days(60));
    let to:DateTime<Utc> = opts.to.parse().unwrap_or(Utc::now());

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(&["period start", "symbol", "price", "change %", "min", "max", "30d avg"]).unwrap();

    for ticker in opts.tickers.split(",").collect::<Vec<&str>>() {
        let mut quotes = fetch_stonks_data(ticker, from, to).unwrap();
        quotes.sort_by_cached_key(|k| k.timestamp); //just in case aren't sorted already

        let mut adjclose_series = extract_adjclose(&quotes);
        let (min_, max_) = min_and_max(&adjclose_series);
        let smas = n_window_sma(30, &adjclose_series).unwrap();
        let (diff, percent) = price_diff(&adjclose_series);

        let first_quote = &quotes[0];

        wtr.write_record(&[
            Utc.timestamp(first_quote.timestamp as i64, 0).to_rfc3339(),
            ticker.into(),
            first_quote.close.round_dp(2).to_string(),
            (percent * Decimal::from(100)).round_dp(2).to_string(),
            min_.round_dp(2).to_string(),
            max_.round_dp(2).to_string(),
            smas[smas.len() - 1].round_dp(2).to_string(),
        ]).unwrap();
        wtr.flush().unwrap();
    }
}

///API Limits
// - Using the Public API (without authentication), you are limited to 2,000 requests per hour per IP (or up to a total of 48,000 requests a day).
// - Seems that 1h is resolution limit
fn fetch_stonks_data(ticker: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<YQuote>, Box<dyn Error>> {
    let provider = yahoo_finance_api::YahooConnector::new();
    //todo frequency - play with 1d/1h should work for both
    let response = match provider.get_quote_history_interval(ticker, from, to, "1d") {
        Ok(r) => r,
        Err(e) => {
            println!("An ERROR occured: {:?}", e);
            return Err(Box::new(e));
        }
    };
    let quotes: Vec<YQuote> = response.quotes().unwrap().into_iter().map(|q| q.into()).collect();
    // for quote in &quotes {
    //     println!("{:#?}", quote);
    //     let time_of_quote = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    //     println!("{}", time_of_quote);
    // }
    Ok(quotes)
}

fn extract_adjclose(quotes: &[YQuote]) -> Vec<Decimal> {
    quotes.iter().map(|q| q.adjclose).collect()
}

fn min_and_max(series: &[Decimal]) -> (Decimal, Decimal) {
    // todo still think this can be done better than O(n) - will see if it's a bottleneck
    let mut min_ = series[0];
    let mut max_ = series[0];
    for price in series {
        if *price < min_ {
            min_ = *price
        } else if *price > max_ {
            max_ = *price
        }
    }
    (min_, max_)
}


/// only calculates when enough days
fn n_window_sma(n: usize, series: &[Decimal]) -> Option<Vec<Decimal>> {
    if n > series.len() {
        return None;
    }
    let smas: Vec<Decimal> = series //todo
        .windows(n)
        .map(|w| w.iter().sum::<Decimal>() / Decimal::from(n))
        .collect();
    Some(smas)
}

fn price_diff(series: &[Decimal]) -> (Decimal, Decimal) {
    let mut first = series[0];
    // prevent division by 0
    if first == Decimal::from(0) {
        first = Decimal::from(1)
    }
    let last = series[series.len() - 1];
    (last - first, last / first - Decimal::from(1))
}

