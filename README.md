# `news-ticker`

[![ci](https://github.com/rvhonorato/news-ticker/actions/workflows/ci.yml/badge.svg)](https://github.com/rvhonorato/news-ticker/actions/workflows/ci.yml)
![Crates.io Version](https://img.shields.io/crates/v/news-ticker)

This is an overly complicated way for me to keep up with the news.

`news-ticker` is a command-line RSS feed reader that fetches news from some
feeds, stores entries in a local SQLite database, and displays them one at a
time.

## Content Filter ( Offensive Content Detection )

news-ticker includes an optional content filter that uses Ollama to detect
potentially offensive or distressing content in news titles. When offensive
content is detected, the title is prefixed with `[TW]` (Trigger Warning).

### How It Works

- The content filter uses a local Ollama instance with a language model
- It is **opt-in**: pass `--model <MODEL>` to enable it. Without `--model`,
  no filtering is applied
- If Ollama is not available or the model fails to load, the filter is **skipped
  automatically** and news is displayed without filtering

### Configuration

The content filter connects to Ollama on `http://localhost:11434` (not
currently configurable). Specify which model to use via the `--model` flag:

```sh
news-ticker --model llama3.2:3b --refresh feeds.txt
```

### Model Requirements

Any Ollama model can be used, but it should be fine-tuned or capable of binary
classification. The filter works best with models that can respond with short,
deterministic output.

Make sure the model is pulled in Ollama before using it:

```sh
ollama pull llama3.2:3b
```

### Content Classification Criteria

See the PROMPT inside `filter::build_classification_prompt` for more details.

## Usage

Refresh feed data from a file (one URL per line):

```sh
news-ticker --refresh feeds.txt
```

Refresh with content filtering enabled, using a specific Ollama model:

```sh
news-ticker --model llama3.2:3b --refresh feeds.txt
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

Delete all entries:

```sh
news-ticker --purge
```

### Verbose Mode

For debugging content filter issues, use the `-v` flag:

```sh
news-ticker -v --refresh feeds.txt
```

This shows detailed logs about content classification and any errors from Ollama.

## Waybar Integration

Add a custom module to your Waybar `config.jsonc`:

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

### Recommended Crontab

For auto-advance and periodic refresh, add to your crontab (`crontab -e`):

```cron
# Auto-advance to next entry every minute
* * * * * /home/rodrigo/.cargo/bin/news-ticker --next

# Refresh feeds every 20 minutes
*/20 * * * * /home/rodrigo/.cargo/bin/news-ticker --refresh $HOME/feeds.txt
```
