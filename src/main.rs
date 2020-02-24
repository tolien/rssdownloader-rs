extern crate chrono;
#[macro_use]
extern crate log;

extern crate fern;

use fern::colors::{Color, ColoredLevelConfig};
extern crate dirs;

use futures::executor::block_on;
use rss::Channel;
use rssdownloader_rs::Config;
use std::process;

#[tokio::main]
async fn main() {
    let logger_result = setup_logger();
    if logger_result.is_err() {
        panic!("Error applying fern logger");
    }

    let config_result = Config::new();
    let config;
    if config_result.is_ok() {
        config = config_result.unwrap()
    } else {
        error!("Error parsing config: {}", config_result.err().unwrap());
        process::exit(1);
    };

    debug!("Global download dir: {}", config.global_download_dir);
    info!("Working with {} feed(s)", config.feeds.len());

    for feed in config.feeds {
        info!("Fetching {}", feed.name);
        let rss_channel = block_on(fetch_rss(&feed.url)).unwrap();
        for item in rss_channel.into_items() {
            let title = item.title().unwrap();
            debug!("Title: {}", title);
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
            for regex in &feed.download_filter {
                if regex.is_match(title) {
                    info!("title: {:?}", title);
                    debug!("url: {:?}", item.link().unwrap())
                }
            }
        }
    }
}

async fn fetch_rss(url: &str) -> Result<Channel, Box<dyn std::error::Error>> {
    debug!("Fetching URL {}", url);
    let text = reqwest::get(url).await?.text().await?;

    let channel = Channel::read_from(text.as_bytes()).unwrap();

    Ok(channel)
}

fn setup_logger() -> Result<(), fern::InitError> {
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        // we actually don't need to specify the color for debug and info, they are white by default
        .info(Color::White)
        .debug(Color::White)
        // depending on the terminals color scheme, this is the same as the background color
        .trace(Color::BrightBlack);

    let colors_level = colors_line.info(Color::Green);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    colors_line.get_color(&record.level()).to_fg_str()
                ),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Info)
        .level_for("tokio_reactor", log::LevelFilter::Off)
        .level_for("tokio_postgres", log::LevelFilter::Off)
        .level_for("reqwest", log::LevelFilter::Off)
        .level_for("hyper", log::LevelFilter::Off)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
