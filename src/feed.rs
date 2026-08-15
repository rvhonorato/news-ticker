use feed_rs::{model::Entry, parser};
use reqwest::Client;
use sqlx::SqlitePool;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::filter::ContentFilter;

use super::db::{add_entry_to_db, init_current_entry, init_db};
use tracing::{debug, info, warn};

/// Reads feed URLs from a file, one per line
pub fn read_feed_urls(path: &str) -> Result<Vec<String>, String> {
    let feeds_file = Path::new(path);

    if !feeds_file.exists() {
        return Err(format!("Feeds file not found: {}", path));
    }

    let file = File::open(feeds_file).map_err(|e| format!("Unable to open {}: {}", path, e))?;
    let reader = BufReader::new(file);

    let urls: Vec<String> = reader
        .lines()
        .filter_map(|line| {
            let line = line.unwrap_or_default();
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect();

    if urls.is_empty() {
        return Err("No valid URLs found in feeds file".to_string());
    }

    Ok(urls)
}

#[derive(Debug)]
pub struct Fetcher {
    client: Client,
    pub db: SqlitePool,
    pub entries: Vec<Entry>,
    filter: Option<ContentFilter>,
}

impl Fetcher {
    pub async fn new(model: Option<String>) -> Result<Self, String> {
        let db = init_db()
            .await
            .map_err(|e| format!("DB init error: {}", e))?;

        let filter = model.map(|m| ContentFilter::new(m).unwrap());

        Ok(Self {
            client: Client::new(),
            db,
            entries: Vec::new(),
            filter,
        })
    }

    pub async fn refresh_with_urls(&mut self, urls: Vec<String>) -> Result<usize, String> {
        info!("Starting feed refresh for {} URLs", urls.len());
        let mut all_entries = Vec::new();
        for url in &urls {
            match self.fetch_feed(url).await {
                Ok(mut entries) => {
                    debug!("Fetched {} entries from {}", entries.len(), url);
                    all_entries.append(&mut entries);
                }
                Err(e) => {
                    warn!("Failed to fetch {}: {}", url, e);
                }
            }
        }

        self.entries = all_entries.clone();
        info!("Total entries collected: {}", self.entries.len());

        let mut new_inserts = 0;
        for entry in &all_entries {
            if add_entry_to_db(&self.db, entry, self.filter.as_ref()).await? {
                new_inserts += 1;
            }
        }

        // Initialize current entry after adding new entries
        init_current_entry(&self.db).await?;

        info!("Feed refresh complete: {} new entries added", new_inserts);

        Ok(new_inserts)
    }

    async fn fetch_feed(&self, url: &str) -> Result<Vec<Entry>, String> {
        debug!("Fetching feed from {}", url);
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        let feed = parser::parse(body.as_bytes()).map_err(|e| format!("Format error: {}", e))?;

        Ok(feed.entries)
    }
}
