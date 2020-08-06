extern crate chrono;
#[macro_use]
extern crate log;

extern crate dirs;

use reqwest::blocking::Client;
use rss::Channel;
use rssdownloader_rs::{Config, FeedConfig, FetchedItem, SavedState};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::Handle;

extern crate clap;
use clap::{App, Arg};

fn main() {
    let matches = App::new("rssdownloader-rs")
        .arg(
            Arg::with_name("config")
                .short("-c")
                .long("config")
                .help("Path to the config file to use")
                .takes_value(true)
                .required(false),
        )
        .get_matches();

    let logger_handle = bootstrap_logger().unwrap();

    let config_path;
    if let Some(config_path_str) = matches.value_of("config") {
        config_path = Some(PathBuf::from(config_path_str));
    } else {
        config_path = None;
    }

    let config_result = Config::new(config_path);
    if let Ok(config) = config_result {
        let logger_result = apply_config_to_logger(&logger_handle, &config);
        if logger_result.is_err() {
            panic!("Couldn't set up logger");
        }

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
                handle_feed(feed, &client, &config);
            }

            let sleep_time = config.refresh_interval;
            info!("Sleeping for {} seconds", sleep_time.as_secs());
            thread::sleep(sleep_time);
        }
    } else {
        panic!("Error parsing config: {}", config_result.err().unwrap());
    }
}

fn handle_feed(feed: &FeedConfig, client: &Client, config: &Config) {
    let mut saved_state = SavedState::new().unwrap();
    info!("Fetching {}", feed.name);
    let rss_result = fetch_rss(&feed.url, &client);
    if rss_result.is_err() {
        error!("Failed to load RSS feed");
        return;
    }
    let rss_channel = rss_result.unwrap();
    for item in rss_channel.into_items() {
        let title_result = item.title();
        if title_result.is_none() {
            debug!("No title found");
            return;
        }
        let title = title_result.unwrap();
        trace!("Title: {}", title);
        if let Some(global_regex) = &feed.global_include_filter {
            if !global_regex.is_match(title) {
                return;
            }
        }
        if let Some(global_exclude_regex) = &feed.global_exclude_filter {
            if global_exclude_regex.is_match(title) {
                return;
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
                return;
            }

            info!("Matched title: {:?}", title);
            debug!("url: {:?}", item_url);
            let fetch_result = fetch_item(item_url, &client, &config.global_download_dir);
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

fn bootstrap_logger() -> Result<Handle, log4rs::Error> {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "[{d(%Y-%m-%d %H:%M:%S)}][{h({l})}] {m}{n}",
        )))
        .build();

    let stdout_appender = Appender::builder().build("stdout", Box::new(stdout));
    let config = log4rs::config::Config::builder()
        .appender(stdout_appender)
        .logger(
            Logger::builder()
                .appender("stdout")
                .additive(false)
                .build("stdout_log", LevelFilter::Trace),
        )
        .build(Root::builder().appender("stdout").build(LevelFilter::Trace))
        .unwrap();

    let handle = log4rs::init_config(config).unwrap();

    Ok(handle)
}

fn apply_config_to_logger(handle: &Handle, config: &Config) -> Result<(), log4rs::Error> {
    let mut config_builder = log4rs::config::Config::builder();
    let mut root_builder = Root::builder();
    let mut logger_builder = Logger::builder().additive(false);
    let mut max_level = LevelFilter::Off;

    if let Some(log_level) = config.log_level_stdout {
        if log_level > max_level {
            max_level = log_level;
        }
        debug!("Stdout log level: {}", log_level);
        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "[{d(%Y-%m-%d %H:%M:%S)}][{h({l})}] {m}{n}",
            )))
            .build();

        let stdout_appender = Appender::builder()
            .filter(Box::new(ThresholdFilter::new(log_level)))
            .build("stdout", Box::new(stdout));

        config_builder = config_builder.appender(stdout_appender);
        root_builder = root_builder.appender("stdout");
        logger_builder = logger_builder.appender("stdout")
    } else {
        info!("No stdout log level found, this will be one of the last messages logged to stdout");
    }

    if let Some(log_path) = &config.log_file_path {
        if let Some(log_level) = config.log_level_file {
            if log_level > max_level {
                max_level = log_level;
            }
            debug!("File log level: {}", log_level);
            let logfile = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(
                    "[{d(%Y-%m-%d %H:%M:%S)}][{h({l})}] {m}{n}",
                )))
                .build(log_path)
                .unwrap();

            let file_appender = Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log_level)))
                .build("file", Box::new(logfile));

            config_builder = config_builder.appender(file_appender);
            root_builder = root_builder.appender("file");
            logger_builder = logger_builder.appender("file");
        }
    } else {
        info!("No file log level found, won't enable file logging");
    }

    config_builder = config_builder.logger(logger_builder.build("rssdownloader_rs", max_level));
    let root = root_builder.appender("stdout").build(LevelFilter::Off);
    let log4rs_config = config_builder.build(root).unwrap();

    handle.set_config(log4rs_config);

    Ok(())
}
