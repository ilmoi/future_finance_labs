use chrono::{DateTime, Utc, TimeZone};
use std::time::{UNIX_EPOCH, Duration};
use clap::{App, Arg};
use yahoo_finance_api::{Quote};
use std::error::Error;
use std::cmp::Ordering;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::io;

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


fn main() {
    let matches = App::new("future finance labs cli app")
        .version("0.1")
        .about("fetches stonks data")
        .arg(Arg::with_name("tickers")
             // .required(true)
             // .index(1) //not sure does anything
        )
        .arg(Arg::with_name("from")
            .help("Date in yyyy-mm-dd format. Default = 1 month ago.")
            .short("f")
            .long("from")
            .takes_value(true))// this specifies that it's an option, not a flag
        .arg(Arg::with_name("to")
            .help("Date in yyyy-mm-dd format. Default = now.")
            .short("t")
            .long("to")
            .takes_value(true))// this specifies that it's an option, not a flag
        .get_matches();

    let tickers = matches.value_of("tickers").unwrap_or("aapl,msft,uber"); //required so safe to unwrap
    let from = match matches.value_of("from") {
        Some(v) => {
            let vv = v.split("-").collect::<Vec<&str>>();
            Utc.ymd(vv[0].parse::<i32>().unwrap(), vv[1].parse::<u32>().unwrap(), vv[2].parse::<u32>().unwrap()).and_hms(0, 0, 0)
        }
        None => {
            Utc::now() - chrono::Duration::days(60)
        }
    };
    let to = match matches.value_of("to") {
        Some(v) => {
            let vv = v.split("-").collect::<Vec<&str>>();
            Utc.ymd(vv[0].parse::<i32>().unwrap(), vv[1].parse::<u32>().unwrap(), vv[2].parse::<u32>().unwrap()).and_hms(0, 0, 0)
        }
        None => {
            Utc::now()
        }
    };

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(&["period start", "symbol", "price", "change %", "min", "max", "30d avg"]).unwrap();

    for ticker in tickers.split(",").collect::<Vec<&str>>() {
        let quotes = fetch_stonks_data(ticker, from, to).unwrap();
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

/// IMPL FOR FLOATS
/// naive implementation - linearly go through the vector and update min / max
/// I thought about sorting then plucking of first and last values, but that requires vector not slice... not sure how to solve
// fn min_and_max(series: &[f64]) -> Option<(f64, f64)> {
//     // https://rust-lang-nursery.github.io/rust-cookbook/algorithms/sorting.html#sort-a-vector-of-floats
//     // let sorted_series = series.clone_from_slice().sort_by(|a,b| a.partial_cmp(b).unwrap());
//
//     let mut min_ = series[0];
//     let mut max_ = series[0];
//     for price in series {
//         match min_.partial_cmp(price) {
//             None => return None,
//             Some(Ordering::Greater) => min_ = *price,
//             _ => (),
//         }
//
//         match max_.partial_cmp(price) {
//             None => return None,
//             Some(Ordering::Less) => max_ = *price,
//             _ => (),
//         }
//     }
//
//     Some((min_, max_))
// }

/// only calculates when enough days
fn n_window_sma(n: usize, series: &[Decimal]) -> Option<Vec<Decimal>> {
    let mut smas = vec![];

    let series_len = series.len();
    if n > series_len {
        return None;
    }

    let diff = series_len - n;
    for i in 0..diff {
        let local_series = &series[0 + i..n + i];
        //https://codereview.stackexchange.com/questions/173338/calculate-mean-median-and-mode-in-rust/173437
        let avg = local_series.iter().sum::<Decimal>() as Decimal / Decimal::from(n);
        smas.push(avg);
    }

    Some(smas)
}

fn price_diff(series: &[Decimal]) -> (Decimal, Decimal) {
    let first = series[0];
    let last = series[series.len() - 1];
    (last - first, last / first - Decimal::from(1))
}

