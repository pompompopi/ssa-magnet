use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use tracing::info;

#[inline]
pub async fn send(year: i32, client: Arc<Client>) -> Result<(String, i32)> {
    info!("requesting year {year}");
    client
        .post("https://www.ssa.gov/cgi-bin/popularnames.cgi")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("year={}&top=1000&number=n&token=Submit", year))
        .send()
        .await?
        .text()
        .await
        .map(|b| (b, year))
        .map_err(anyhow::Error::new)
}
