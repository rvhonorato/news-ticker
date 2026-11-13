# `news-ticker`

[![ci](https://codeberg.org/rvhonorato/news-ticker/badges/workflows/ci.yml/badge.svg)](https://codeberg.org/rvhonorato/news-ticker/actions?workflow=ci.yml)
[![coverage](https://img.shields.io/badge/dynamic/json?url=https://rvhonorato.codeberg.page/news-ticker/coverage/tarpaulin-report.json&query=$.coverage&label=coverage&suffix=%25)](https://rvhonorato.codeberg.page/news-ticker/coverage/tarpaulin-report.html)
![Crates.io Version](https://img.shields.io/crates/v/news-ticker)

An overly complicated way for me to keep up with the news.

`news-ticker` reads RSS feeds, keeps the entries in a local SQLite database, and shows them one at a time. I use it in my Waybar.

## Install

```sh
cargo install news-ticker
```

## Usage

Put your feeds in a text file, one URL per line. Lines that start with `#` are skipped. See [`feeds.txt`](feeds.txt) for an example.

```sh
news-ticker --refresh feeds.txt   # fetch new entries
news-ticker                       # show the current entry
news-ticker --next                # go to the next entry
news-ticker --prev                # go to the previous entry
news-ticker --link                # show only the link
news-ticker --waybar              # show the entry as Waybar JSON
news-ticker --purge               # delete all entries
news-ticker -v ...                # show debug logs
```

The database is at `~/.cache/news-ticker/db.sqlite`. Set `NEWS_TICKER_DB` to use a different file.

## Trigger warnings

`news-ticker` can put ⚠️ in front of titles about death, violence, accidents, disasters and similar news. It uses a small local [Ollama](https://ollama.com) decision model to do this.

```sh
ollama pull tev1:4b
news-ticker --model tev1:4b --refresh feeds.txt
```

Notes:

- Only new entries are checked, during `--refresh`.
- Normal chat models (for example `llama3.2`) do not work it must use a decision model such as `tev1:4b`.
- If Ollama is not running, you get a warning and the entries are not marked.
- The model is not perfect. It works on Dutch titles too, but it sometimes marks harmless titles with words like "doom" or "crash".
- The question it asks is in `ContentFilter::build_request` in [`src/filter.rs`](src/filter.rs).

## Waybar

Add this module to your Waybar `config.jsonc`:

```jsonc
"custom/news-ticker": {
  "exec": "news-ticker --waybar",
  "format": "📰 {}",
  "return-type": "json",
  "max-length": 60,
  "interval": 5,
  "on-click": "xdg-open \"$(news-ticker --link)\"",
  "on-scroll-up": "news-ticker --next",
  "on-scroll-down": "news-ticker --prev"
}
```

To go to the next entry every minute and fetch feeds every 20 minutes, add this to your crontab (`crontab -e`):

```cron
* * * * * $HOME/.cargo/bin/news-ticker --next
*/20 * * * * $HOME/.cargo/bin/news-ticker --model tev1:4b --refresh $HOME/feeds.txt
```

## Contributing

Issues and pull requests are welcome on [Codeberg](https://codeberg.org/rvhonorato/news-ticker). Before you open a pull request, make sure that these pass:

```sh
cargo test
cargo clippy -- -D warnings
```

## License

[0BSD](LICENSE). Do what you want with it.
