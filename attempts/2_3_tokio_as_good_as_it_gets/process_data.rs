use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::download_data::YQuote;
use chrono::{TimeZone, Utc};
use std::io;
use std::time::Duration;

pub struct ProcessedData {
    pub min_: Decimal,
    pub max_: Decimal,
    pub smas: Vec<Decimal>,
    pub abs_diff: Decimal,
    pub percent_diff: Decimal,
}

pub fn extract_adjclose(quotes: &[YQuote]) -> Vec<Decimal> {
    quotes.iter().map(|q| q.adjclose).collect()
}

pub fn min_and_max(series: &[Decimal]) -> (Decimal, Decimal) {
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
pub fn n_window_sma(n: usize, series: &[Decimal]) -> Option<Vec<Decimal>> {
    if n > series.len() {
        return None;
    }
    let smas: Vec<Decimal> = series
        .windows(n)
        .map(|w| w.iter().sum::<Decimal>() / Decimal::from(n))
        .collect();
    Some(smas)
}

pub fn price_diff(series: &[Decimal]) -> (Decimal, Decimal) {
    let mut first = series[0];
    // prevent division by 0
    if first == Decimal::from(0) {
        first = Decimal::from(1)
    }
    let last = series[series.len() - 1];
    (last - first, last / first - Decimal::from(1))
}

pub fn process_data(quotes: Vec<YQuote>, ticker: String) -> ProcessedData {
    println!("START processing...");

    std::thread::sleep(Duration::from_secs(5));
    // tokio::time::sleep(tokio::time::Duration::from_secs(5)); // <-- won't work inside a normal (non async) fn

    let ts = quotes[0].timestamp;
    let close = quotes[0].close;

    let mut adjclose_series = extract_adjclose(&quotes);
    let (min_, max_) = min_and_max(&adjclose_series);
    let smas = n_window_sma(30, &adjclose_series).unwrap();
    let (abs_diff, percent_diff) = price_diff(&adjclose_series);

    println!("END processing...");

    // write output
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(&[
        Utc.timestamp(ts as i64, 0).to_rfc3339(),
        ticker.into(),
        close.round_dp(2).to_string(),
        (percent_diff * Decimal::from(100)).round_dp(2).to_string(),
        min_.round_dp(2).to_string(),
        max_.round_dp(2).to_string(),
        smas[smas.len() - 1].round_dp(2).to_string(),
    ])
    .unwrap();
    wtr.flush().unwrap();

    ProcessedData {
        min_,
        max_,
        smas,
        abs_diff,
        percent_diff,
    }
}
