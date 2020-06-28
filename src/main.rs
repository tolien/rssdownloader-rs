extern crate chrono;
#[macro_use]
extern crate log;

extern crate dirs;

use reqwest::blocking::Client;
use rss::Channel;
use rssdownloader_rs::{Config, FetchedItem, SavedState};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

fn main() {
    let config_result = Config::new();
    if let Ok(config) = config_result {
        let mut saved_state = SavedState::new().unwrap();

        debug!(
            "Global download dir: {}",
            config.global_download_dir.to_str().unwrap()
        );
        debug!("Working with {} feed(s)", config.feeds.len());
        let client = Client::builder()
            .gzip(true)
            .connect_timeout(Duration::new(5, 0))
            .timeout(Duration::new(5, 0))
            .connection_verbose(true)
            .build()
            .unwrap();

        loop {
            for feed in &config.feeds {
                info!("Fetching {}", feed.name);
                let rss_result = fetch_rss(&feed.url, &client);
                if rss_result.is_err() {
                    error!("Failed to load RSS feed");
                    continue;
                }
                let rss_channel = rss_result.unwrap();
                for item in rss_channel.into_items() {
                    let title_result = item.title();
                    if title_result.is_none() {
                        debug!("No title found");
                        continue;
                    }
                    let title = title_result.unwrap();
                    trace!("Title: {}", title);
                    if let Some(global_regex) = &feed.global_include_filter {
                        if !global_regex.is_match(title) {
                            continue;
                        }
                    }
                    if let Some(global_exclude_regex) = &feed.global_exclude_filter {
                        if global_exclude_regex.is_match(title) {
                            continue;
                        }
                    }
                    if feed.download_filter.is_match(title) {
                        let item_url = item.link().unwrap();

                        let fetched_item = FetchedItem {
                            name: String::from(title),
                            url: String::from(item_url),
                        };

                        if saved_state.fetched_before(&fetched_item).unwrap() {
                            debug!("Skipping previously fetched item {}", title);
                            continue;
                        }

                        info!("Matched title: {:?}", title);
                        debug!("url: {:?}", item_url);
                        let fetch_result =
                            fetch_item(item_url, &client, &config.global_download_dir);
                        if fetch_result.is_ok() {
                            saved_state.save(&fetched_item).unwrap_or_else(|err| {
                                error!("Failed to save state: {:?}", err);
                            });
                        } else {
                            error!("Failed to fetch item: {:?}", fetch_result.err());
                        }
                    }
                }
            }

            let sleep_time = config.refresh_interval;
            info!("Sleeping for {} seconds", sleep_time.as_secs());
            thread::sleep(sleep_time);
        }
    } else {
        panic!("Error parsing config: {}", config_result.err().unwrap());
    }
}

fn fetch_rss(url: &str, client: &Client) -> Result<Channel, Box<dyn std::error::Error>> {
    debug!("Fetching URL {}", url);
    let response = client.get(url).send()?;
    let status = response.status();
    info!("Response status: {}", status);
    if status.is_success() {
        let text = response.text()?;

        let rss_result = Channel::read_from(text.as_bytes());
        if let Ok(channel) = rss_result {
            Ok(channel)
        } else {
            let error = rss_result.err().unwrap();
            error!("Error parsing RSS feed: {:?}", error);
            Err(Box::new(error))
        }
    } else {
        error!("Error fetching RSS feed");
        Err(Box::new(response.error_for_status().err().unwrap()))
    }
}

fn fetch_item(
    url: &str,
    client: &Client,
    destination_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Fetching item {}", url);
    let response = client.get(url).send()?;
    if response.status().is_success() {
        let headers = response.headers();
        let content_dispo = headers.get("content-disposition");

        let mut filename = String::new();
        if let Some(dispo_text) = content_dispo {
            let dispo_parts = dispo_text.to_str().unwrap().split(';');
            for part in dispo_parts {
                if part.trim().starts_with("filename=") {
                    filename = part.trim().replace("filename=", "").replace("\"", "");
                }
            }
            debug!("Using filename: {:?}", filename);
        }
        let mut dest = PathBuf::from(destination_dir);
        if !dest.exists() {
            fs::create_dir(&dest)?;
        }
        dest.push(filename);
        if dest.exists() {
            info!("Not overwriting existing item");
        } else {
            fs::write(dest, response.bytes()?)?;
        }
    }

    Ok(())
}

