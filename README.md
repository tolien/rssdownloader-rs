# rssdownloader-rs
![Rust](https://github.com/tolien/rssdownloader-rs/workflows/Rust/badge.svg)
[![dependency status](https://deps.rs/repo/github/tolien/rssdownloader-rs/status.svg)](https://deps.rs/repo/github/tolien/rssdownloader-rs)

A Rust-based RSS feed parser

## Configuration

A configuration file is expected. This should be *config.toml*
within a directory called .rssdownloader-rs in your home directory, i.e.:

* $HOME/.rssdownloader-rs/config.toml on Linux/macOS
* %USERPROFILE%\.rssdownloader-rs\config.toml on Windows

An example config file would look like this:

```
download_dir=/opt/podcasts

[feeds]
  [feeds.feed_name]
  feed_url="http://url.to.podcast/stuff_and_things.html"
  download_regex_list = [
    '.'
  ]
```

Additionally, you can add these options for a feed to simplify your download regexes:

* feed_regex - items which don't match this should be skipped regardless of if they match a download regex
* feed_skip_regex in a feed - regex for a feed for items which should be skipped
