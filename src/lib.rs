use std::fs;

use regex::Regex;

use toml::Value;
#[macro_use]
extern crate log;

extern crate fern;

pub struct FeedConfig {
    pub name: String,
    pub url: String,
    pub global_include_filter: Option<Regex>,
    pub global_exclude_filter: Option<Regex>,
    pub download_filter: Vec<Regex>,
}
impl FeedConfig {
    pub fn new(name: &str, values: &toml::Value) -> Result<Self, &'static str> {
        let url;
        if let Some(url_value) = values.get("feedurl") {
            url = url_value.as_str().unwrap();
            debug!("feed URL: {}", String::from(url));
        } else {
            return Err("No URL found for feed");
        }

        let feed_filter;
        if values.get("feed_regex").is_some() {
            let filter = values.get("feed_regex").unwrap();
            if filter.is_str() {
                let filter_string = filter.as_str().unwrap();
                feed_filter = Some(Regex::new(filter_string).unwrap());
            } else {
                feed_filter = None;
            }
        } else {
            feed_filter = None;
        }

        let feed_skip_filter;
        if values.get("feed_skip_regex").is_some() {
            let filter = values.get("feed_skip_regex").unwrap();
            if filter.is_str() {
                let filter_string = filter.as_str().unwrap();
                feed_skip_filter = Some(Regex::new(filter_string).unwrap());
            } else {
                feed_skip_filter = None;
            }
        } else {
            feed_skip_filter = None;
        }

        let mut regex_list = Vec::new();
        let feed_filters = values.get("download_regex_list");
        if let Some(filters) = feed_filters {
            for filter in filters.as_array().unwrap() {
                if filter.as_str().is_some() {
                    regex_list.push(Regex::new(filter.as_str().unwrap()).unwrap());
                }
            }
        }

        info!("feed regex list size: {}", regex_list.len());

        Ok(Self {
            name: String::from(name),
            url: String::from(url),
            global_include_filter: feed_filter,
            global_exclude_filter: feed_skip_filter,
            download_filter: regex_list,
        })
    }
}

pub struct Config {
    pub global_download_dir: String,
    pub feeds: Vec<FeedConfig>,
}
impl Config {
    pub fn new() -> Result<Self, &'static str> {
        let working_dir = dirs::home_dir().unwrap().join(".rssdownloader-rs");
        let config_path = working_dir.join("config.toml");
        debug!("Using config path {:?}", config_path);
        if let Ok(properties) = fs::read_to_string(config_path) {
            Config::construct_from_string(&properties)
        } else {
            Err("Couldn't open config file")
        }
    }

    fn construct_from_string(properties: &str) -> Result<Self, &'static str> {
        let values = properties.parse::<Value>().unwrap();
        let feeds = values["feeds"].as_table().unwrap();
        println!("Feeds found: {}", feeds.len());
        let mut feed_objects = Vec::<FeedConfig>::new();
        for feed in feeds.keys() {
            println!("feed name: {:?}", feed);
            if let Some(feed_value) = feeds.get(feed) {
                let feed_obj = FeedConfig::new(feed, feed_value);
                if feed_obj.is_ok() {
                    feed_objects.push(FeedConfig::new(feed, feed_value).unwrap());
                } else if let Some(error) = feed_obj.err() {
                    println!("Error parsing config: {}", error);
                }
            }
        }

        Ok(Self {
            global_download_dir: values["downloadDir"].to_string(),
            feeds: feed_objects,
        })
    }
}
