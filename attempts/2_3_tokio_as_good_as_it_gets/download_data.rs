use std::error::Error;

use crate::process_data::min_and_max;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::sync::Arc;
use yahoo_finance_api::{Quote, YahooConnector};

#[derive(Debug, PartialEq, PartialOrd)]
pub struct YQuote {
    pub timestamp: u64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub volume: u64,
    pub close: Decimal,
    pub adjclose: Decimal,
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

///API Limits
// - Using the Public API (without authentication), you are limited to 2,000 requests per hour per IP (or up to a total of 48,000 requests a day).
// - Seems that 1h is resolution limit
pub async fn fetch_stonks_data(
    ticker: String,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<YQuote>, Box<dyn Error>> {
    println!("START downloading...");

    let provider = YahooConnector::new();

    std::thread::sleep(std::time::Duration::from_secs(1));

    //todo frequency - play with 1d/1h should work for both
    let response = match provider
        .get_quote_history_interval(&ticker, from, to, "1d")
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("An ERROR occured: {:?}", e);
            return Err(Box::new(e));
        }
    };
    let mut quotes: Vec<YQuote> = response
        .quotes()
        .unwrap()
        .into_iter()
        .map(|q| q.into())
        .collect();

    // for quote in &quotes {
    //     println!("{:#?}", quote);
    //     let time_of_quote = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    //     println!("{}", time_of_quote);
    // }

    quotes.sort_by_cached_key(|k| k.timestamp); //just in case aren't sorted already

    println!("END downloading...");

    Ok(quotes)
}
