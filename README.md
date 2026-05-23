# `news-ticker`

[![ci](https://github.com/rvhonorato/news-ticker/actions/workflows/ci.yml/badge.svg)](https://github.com/rvhonorato/news-ticker/actions/workflows/ci.yml)
![Crates.io Version](https://img.shields.io/crates/v/news-ticker)


This is an overly complicated way for me to keep up with the news.

`news-ticker` is a command-line RSS feed reader that fetches news from some
feeds, stores entries in a local SQLite database, and displays them one at a
time.

It supports navigation between entries, Waybar status bar integration, and
URL-only output for scripting workflows. Feed URLs are read from a plain text
file (one per line), and the database persists fetched entries between runs.

## Usage

Refresh feed data from a file (one URL per line):

```sh
news-ticker --refresh feeds.txt
```

Display the current news entry:

```sh
news-ticker
```

Display only the URL of the current entry:

```sh
news-ticker --link
```

Navigate entries:

```sh
news-ticker --next   # Go to next entry
news-ticker --prev   # Go to previous entry (alias for --previous)
```

Output in Waybar JSON format:

```sh
news-ticker --waybar
```

## Waybar Integration

todo
