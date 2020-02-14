extern crate dirs;

use std::fs;
use futures::executor::block_on;
use regex::Regex;
use rss::Channel;
use toml::Value;

#[tokio::main]
async fn main() {
    let config = Config::new().unwrap();
    println!("Global download dir: {}", config.global_download_dir);

    for feed in config.feeds {
        println!("Fetching {}", feed.name);
        let rss_channel = block_on(fetch_rss(&feed.url)).unwrap();
        for item in rss_channel.into_items() {
            let title = item.title().unwrap();
            for regex in &feed.download_filter {
                if regex.is_match(title) {
                    println!("title: {:?}", title);
                    println!("url: {:?}", item.link().unwrap())
                }
            }
        }
    }
}

async fn fetch_rss(url: &str) -> Result<Channel, Box<dyn std::error::Error>> {
    let text = reqwest::get(url).await?.text().await?;

    let channel = Channel::read_from(text.as_bytes()).unwrap();

    Ok(channel)
}

pub struct FeedConfig {
    pub name: String,
    pub url: String,
    pub global_filter: Option<Regex>,
    pub download_filter: Vec<Regex>
}
impl FeedConfig {
    pub fn new(name: &str, values: &toml::Value) -> Result<Self, &'static str> {
        let url = values["feedurl"].as_str().unwrap_or_else(|| {
            return "No URL found for feed";
        });
        println!("feed URL: {}", String::from(url));

        let feed_filter;
        if values.get("feed_regex").is_some() {
            let filter = values.get("feed_regex").unwrap();
            if filter.is_str() {
                let filter_string = filter.as_str().unwrap();
                feed_filter = Some(Regex::new(filter_string).unwrap());
            }
            else {
                feed_filter = None;
            }
        }
        else {
            feed_filter = None;
        }

        let mut regex_list = Vec::new();
        let feed_filters = values.get("download_regex_list");
        if let Some (filters) = feed_filters {
            for filter in filters.as_array().unwrap() {
                if filter.as_str().is_some() {
                    regex_list.push(Regex::new(filter.as_str().unwrap()).unwrap());
                }
            }
        }

        println!("feed regex list size: {}", regex_list.len());


        Ok(Self {
            name: String::from(name),
            url: String::from(url),
            global_filter: feed_filter,
            download_filter: regex_list
        })
    }
}

pub struct Config {
    pub global_download_dir: String,
    pub feeds: Vec<FeedConfig>
}
impl Config {
    pub fn new() -> Result<Self, &'static str>{
        let working_dir = dirs::home_dir().unwrap().join(".rssdownloader-rs");
        let config_path = working_dir.join("config.toml");
        println!("Using config path {:?}", config_path);
        let properties = fs::read_to_string(config_path)
            .unwrap_or_else(|_err| fs::read_to_string("config.toml").unwrap());
        let values = &properties.parse::<Value>().unwrap();
        let feeds = values["feeds"].as_table().unwrap();
        println!("Feeds found: {}", feeds.len());
        let mut feed_objects = Vec::<FeedConfig>::new();
        for feed in feeds.keys() {
            println!("feed name: {:?}", feed);
            let feed_value = feeds.get(feed).unwrap();
            feed_objects.push(FeedConfig::new(feed, feed_value).unwrap());
        }

        Ok(Self {
            global_download_dir: values["downloadDir"].to_string(),
            feeds: feed_objects
        })


    }
}
