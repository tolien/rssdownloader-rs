use std::fs;

use regex::Regex;
use regex::RegexSet;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::Duration;
use toml::Value;
#[macro_use]
extern crate log;

pub struct FeedConfig {
    pub name: String,
    pub url: String,
    pub global_include_filter: Option<Regex>,
    pub global_exclude_filter: Option<Regex>,
    pub download_filter: RegexSet,
}
impl FeedConfig {
    pub fn new(name: &str, values: &Value) -> Result<Self, &'static str> {
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
    pub global_download_dir: PathBuf,
    pub refresh_interval: Duration,
    pub feeds: Vec<FeedConfig>,
    pub log_file_path: Option<PathBuf>,
    pub log_level_file: Option<log::LevelFilter>,
    pub log_level_stdout: Option<log::LevelFilter>,
}

impl Config {
    pub fn new(path_to_config: Option<PathBuf>) -> Result<Self, &'static str> {
        let config_path = path_to_config.map_or_else(
            || {
                let working_dir = dirs::home_dir().unwrap().join(".rssdownloader-rs");
                working_dir.join("config.toml")
            },
            |path| path,
        );

        debug!("Using config path {:?}", config_path);
        if let Ok(properties) = fs::read_to_string(config_path) {
            Config::construct_from_string(&properties)
        } else {
            Err("Couldn't open config file")
        }
    }

    pub fn construct_from_string(properties: &str) -> Result<Self, &'static str> {
        #![allow(clippy::cast_sign_loss)]
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
                    info!(
                        "Adding feed {} with {} regexes",
                        feed,
                        feed_obj.download_filter.len()
                    );
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

        let mut sleep_interval = Duration::new(12 * 60 * 60, 0);
        if let Some(sleep_value) = values.get("refresh_interval_mins") {
            let value = sleep_value.as_integer().unwrap();
            if value > 0 {
                sleep_interval = Duration::new(60 * value as u64, 0);
            }
        } else {
            info!("Refresh interval not specified, defaulting to 12 hours");
        }

        let mut log_file_path = None;
        if let Some(log_dir) = values.get("log_dir") {
            let mut log_file = PathBuf::from(log_dir.as_str().unwrap());
            log_file.push("rss.log");
            log_file_path = Some(log_file);
        }

        let mut log_level_stdout = None;
        if let Some(log_level_name) = values.get("log_level_stdout") {
            log_level_stdout =
                Config::convert_string_to_levelfilter(log_level_name.as_str().unwrap());
        }

        let mut log_level_file = None;
        if let Some(log_level_name) = values.get("log_level_file") {
            log_level_file =
                Config::convert_string_to_levelfilter(log_level_name.as_str().unwrap());
        }

        Ok(Self {
            global_download_dir: PathBuf::from(download_dir),
            refresh_interval: sleep_interval,
            feeds: feed_objects,
            log_file_path,
            log_level_stdout,
            log_level_file,
        })
    }

    fn convert_string_to_levelfilter(level: &str) -> Option<log::LevelFilter> {
        match level {
            "Info" => Some(log::LevelFilter::Info),
            "Error" => Some(log::LevelFilter::Error),
            "Debug" => Some(log::LevelFilter::Debug),
            "Trace" => Some(log::LevelFilter::Trace),
            _ => None,
        }
    }
}

pub struct FetchedItem {
    pub name: String,
    pub url: String,
}

pub struct SavedState {
    db_connection: Connection,
}

impl SavedState {
    pub fn new() -> Result<Self, &'static str> {
        let connection = SavedState::open_state_db().unwrap();

        Ok(Self {
            db_connection: connection,
        })
    }

    fn open_state_db() -> Result<Connection, rusqlite::Error> {
        let working_dir = dirs::home_dir().unwrap().join(".rssdownloader-rs");
        let path = working_dir.join("savedstate.sqlite");

        let db = Connection::open(&path)?;
        SavedState::create_state_table(&db).unwrap_or_else(|err| {
            panic!("Failed to create saved state table: {}", err);
        });
        Ok(db)
    }

    fn create_state_table(db: &Connection) -> Result<(), rusqlite::Error> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS saved_state (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL,
                name TEXT NOT NULL
                )",
            params![],
        )?;

        Ok(())
    }

    pub fn save(&mut self, new_fetch: &FetchedItem) -> Result<(), rusqlite::Error> {
        self.db_connection
            .execute(
                "INSERT INTO saved_state (url, name)
            VALUES (?1, ?2)",
                [&new_fetch.url, &new_fetch.name],
            )
            .unwrap();

        Ok(())
    }

    pub fn fetched_before(&mut self, new_fetch: &FetchedItem) -> Result<bool, rusqlite::Error> {
        let mut statement = self
            .db_connection
            .prepare("SELECT * FROM saved_state WHERE url = ?1")
            .unwrap();

        let result = statement.exists([&new_fetch.url]).unwrap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    use std::time::Duration;

    #[test]
    fn invalid_config() {
        let mut result;

        result = Config::construct_from_string("jibberish");
        assert_eq!(result.err(), Some("Error parsing config file"));

        result = Config::construct_from_string("");
        assert_eq!(result.err(), Some("Feed list not found"));

        result = Config::construct_from_string("download_dir = \"/home/user/download\"\n\n");
        assert_eq!(result.err(), Some("Feed list not found"));
    }

    #[test]
    fn config_is_parsed_correctly() {
        let valid_config = "
        download_dir=\"/tmp/rssdownload/\"
        refresh_interval_mins = 30
        log_dir=\"\"
        log_level_stdout=\"Debug\"

        [feeds]
          [feeds.feed_name]
          feedurl=\"https://example.com/feed.xml\"
          download_regex_list = [
            '.'
          ]
        ";

        let result = Config::construct_from_string(valid_config);
        assert!(result.is_ok());
        let parsed_config = result.unwrap();
        assert_eq!(
            "/tmp/rssdownload/",
            parsed_config.global_download_dir.to_str().unwrap()
        );
        assert_eq!(Duration::new(30 * 60, 0), parsed_config.refresh_interval);
        assert!(parsed_config.log_file_path.is_some());
        assert_eq!(
            parsed_config.log_level_stdout,
            Some(log::LevelFilter::Debug)
        );
        assert_eq!(parsed_config.log_level_file, None);
        assert_eq!(
            "rss.log",
            parsed_config.log_file_path.unwrap().to_str().unwrap()
        );
        assert_eq!(1, parsed_config.feeds.len());

        let parsed_feed = &parsed_config.feeds[0];
        assert_eq!("https://example.com/feed.xml", parsed_feed.url);
        assert_eq!(1, parsed_feed.download_filter.len())
    }
}
