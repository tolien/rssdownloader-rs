use std::fs;

use regex::Regex;
use regex::RegexSet;

use toml::Value;
#[macro_use]
extern crate log;

extern crate fern;

pub struct FeedConfig {
    pub name: String,
    pub url: String,
    pub global_include_filter: Option<Regex>,
    pub global_exclude_filter: Option<Regex>,
    pub download_filter: RegexSet,
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

        let feed_filters = values.get("download_regex_list");
        let mut regex_list = Vec::new();
        if let Some(filters) = feed_filters {
            for filter in filters.as_array().unwrap() {
                if filter.as_str().is_some() {
                    regex_list.push(filter.as_str().unwrap());
                }
            }
        }
        let regex_set = RegexSet::new(regex_list).unwrap();

        info!("feed regex list size: {}", regex_set.len());

        Ok(Self {
            name: String::from(name),
            url: String::from(url),
            global_include_filter: feed_filter,
            global_exclude_filter: feed_skip_filter,
            download_filter: regex_set,
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

    pub fn construct_from_string(properties: &str) -> Result<Self, &'static str> {
        let parse_result = properties.parse::<Value>();
        if parse_result.is_err() {
            error!("Parse error: {:?}", parse_result.err());
            return Err("Error parsing config file");
        };
        let values = parse_result.unwrap();
        let feeds_value = values.get("feeds");
        if feeds_value.is_none() {
            return Err("Feed list not found");
        }
        let feeds_table = feeds_value.unwrap().as_table();
        if feeds_table.is_none() {
            return Err("Feed list not found");
        }
        let feeds = feeds_table.unwrap();
        debug!("Feeds found: {}", feeds.len());
        let mut feed_objects = Vec::<FeedConfig>::new();
        for feed in feeds.keys() {
            if let Some(feed_value) = feeds.get(feed) {
                let feed_obj_result = FeedConfig::new(feed, feed_value);
                if let Ok(feed_obj) = feed_obj_result {
                    info!("Adding feed {}", feed);
                    feed_objects.push(feed_obj);
                } else if let Some(error) = feed_obj_result.err() {
                    error!("Error parsing config: {}", error);
                }
            }
        }

        let download_dir_result = values.get("download_dir");
        if download_dir_result.is_none() {
            return Err("Download directory must be specified");
        }
        let download_dir = download_dir_result.unwrap().as_str().unwrap();

        Ok(Self {
            global_download_dir: String::from(download_dir),
            feeds: feed_objects,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::Config;

    #[test]
    fn config_is_parsed_correctly() {
        let mut result;

        result = Config::construct_from_string("jibberish");
        assert_eq!(result.err(), Some("Error parsing config file"));

        result = Config::construct_from_string("");
        assert_eq!(result.err(), Some("Feed list not found"));

        result = Config::construct_from_string("download_dir = \"/home/user/download\"\n\n");
        assert_eq!(result.err(), Some("Feed list not found"));
    }
}
