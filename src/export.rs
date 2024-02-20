use anyhow::{anyhow, bail, Result};
use scraper::{Html, Selector};
use serde::Serialize;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct Name {
    pub rank: usize,
    pub name: String,
    pub uses: usize,
}

#[derive(Debug, Serialize)]
pub struct YearNames {
    pub male: Vec<Name>,
    pub female: Vec<Name>,
}

impl YearNames {
    pub async fn parse_request(body: String) -> Result<Self> {
        let document = Html::parse_document(&body);
        // TODO: reuse this value across threads
        let selector = Selector::parse("tr[align=\"right\"] > td")
            .map_err(|_| anyhow!("failed to create selector"))?;
        let mut row_iterator = document.select(&selector).map(|e| e.inner_html());
        let mut male = Vec::new();
        let mut female = Vec::new();
        loop {
            match row_iterator.next_chunk::<5>() {
                Ok(rows) => {
                    if !rows[1].is_empty() {
                        male.push(Name {
                            rank: usize::from_str_radix(&rows[0], 10)?,
                            name: rows[1].clone(),
                            uses: usize::from_str_radix(&rows[2].replace(',', ""), 10)?,
                        });
                    }

                    if !rows[3].is_empty() {
                        female.push(Name {
                            rank: usize::from_str_radix(&rows[0], 10)?,
                            name: rows[3].clone(),
                            uses: usize::from_str_radix(&rows[4].replace(',', ""), 10)?,
                        });
                    }
                }
                Err(extra) => {
                    if extra.is_empty() {
                        break;
                    } else {
                        bail!(
                            "returned table rows contains ≈{} more td elements than expected",
                            extra.size_hint().0
                        )
                    }
                }
            }
        }

        info!("parsed {} names", male.len() + female.len());

        Ok(YearNames { male, female })
    }
}
