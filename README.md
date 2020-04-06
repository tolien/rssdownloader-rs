# rssdownloader-rs
![Rust](https://github.com/tolien/rssdownloader-rs/workflows/Rust/badge.svg)
[![dependency status](https://deps.rs/repo/github/tolien/rssdownloader-rs/status.svg)](https://deps.rs/repo/github/tolien/rssdownloader-rs)
[![Coverage Status](https://coveralls.io/repos/github/tolien/rssdownloader-rs/badge.svg?branch=master)](https://coveralls.io/github/tolien/rssdownloader-rs?branch=master)

A Rust-based RSS feed parser

## Configuration

A configuration file is expected. This should be *config.toml*
within a directory called .rssdownloader-rs in your home directory, i.e.:

* $HOME/.rssdownloader-rs/config.toml on Linux/macOS
* %USERPROFILE%\.rssdownloader-rs\config.toml on Windows

An example config file would look like this:

```
download_dir=/opt/podcasts
refresh_interval_mins = 30

[feeds]
  [feeds.feed_name]
  feed_url="http://url.to.podcast/stuff_and_things.html"
  download_regex_list = [
    '.'
  ]
```

The option names should speak for themselves, but this is going to scan an RSS feed every 30 minutes and save anything which matches the regex to /opt/podcasts.

If refresh_interval_mins is not set, the default is 12 hours.

Additionally, you can add these options for a feed to simplify your download regexes:

* feed_regex - items which don't match this should be skipped regardless of if they match a download regex
* feed_skip_regex in a feed - regex for a feed for items which should be skipped

## Execution
Either clone this repository and run directly from the checkout with

    cargo run --release

or build a binary (eventually releases will provide a compiled binary) with

    cargo build --release

and place the binary (which will be target/release/rssdownloader-rs) wherever you'd like before running it with

    ./rssdownloader-rs
