extern crate dirs;

use futures::executor::block_on;
use rssdownloader_rs::Config;
use rss::Channel;
use std::process;

#[tokio::main]
async fn main() {
    let config_result = Config::new();
		let config;
		if config_result.is_ok() {
			config = config_result.unwrap()
		}
		else {
    	println!("Error parsing config: {}", config_result.err().unwrap());
			process::exit(1);			
    };
		
    println!("Global download dir: {}", config.global_download_dir);
    println!("Working with {} feed(s)", config.feeds.len());

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
    println!("Fetching URL {}", url);
    let text = reqwest::get(url).await?.text().await?;

    let channel = Channel::read_from(text.as_bytes()).unwrap();

    Ok(channel)

}
