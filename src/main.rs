#![feature(exact_size_is_empty)]
#![feature(iter_next_chunk)]
#![feature(slice_as_chunks)]

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use export::YearNames;
use futures::future::try_join_all;
use request::send;
use reqwest::Client;
use tokio::{spawn, time::sleep};
use tracing_subscriber::fmt;

mod export;
mod request;

#[derive(Debug, Parser)]
struct Args {
    /// Start year
    #[arg(short, long, default_value = "1880")]
    start_year: i32,
    /// End year
    #[arg(short, long, default_value = "2020")]
    end_year: i32,
    /// File name
    #[arg(short, long, default_value = "scraped.json")]
    output: String,
    /// User agent
    #[arg(
        short,
        long,
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36 Edg/121.0.0."
    )]
    user_agent: String,
    /// Request clump size
    #[arg(short, long, default_value = "10")]
    request_clump_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    fmt().init();
    let args = Args::parse();
    if args.start_year > args.end_year {
        bail!("start year is after end year");
    }

    let mut fetch_requests = Vec::with_capacity((args.end_year - args.start_year) as usize);
    let client_arc = Arc::new(
        Client::builder()
            .user_agent(args.user_agent)
            .build()
            .with_context(|| anyhow!("could not construct http client"))?,
    );

    for year in args.start_year..=args.end_year {
        fetch_requests.push(send(year, client_arc.clone()));
    }

    let mut parse_futures = Vec::with_capacity(fetch_requests.len());
    let delay = Duration::from_secs(3);
    while !fetch_requests.is_empty() {
        let results = try_join_all(
            fetch_requests.drain(0..args.request_clump_size.min(fetch_requests.len())),
        )
        .await
        .with_context(|| anyhow!("could not complete SSA scrape clump"))?;
        for (result, year) in results {
            parse_futures.push((
                spawn(async move { YearNames::parse_request(result).await }),
                year,
            ))
        }
        sleep(delay).await;
    }

    let mut year_names = BTreeMap::new();
    for (future, year) in parse_futures {
        year_names.insert(
            year,
            future
                .await
                .with_context(|| anyhow!("could not spawn thread"))?
                .with_context(|| anyhow!("could not parse data from SSA"))?,
        );
    }

    let mut buf_writer = BufWriter::new(File::create(Path::new(&args.output))?);
    serde_json::to_writer(&mut buf_writer, &year_names)
        .with_context(|| anyhow!("could not serialize data"))?;
    buf_writer
        .flush()
        .with_context(|| anyhow!("could not flush file buffer"))?;

    Ok(())
}
