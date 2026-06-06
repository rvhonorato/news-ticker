use clap::Parser;
use news_ticker::db::{advance_to_next, get_current, go_to_previous, purge_db};
use news_ticker::feed::{Fetcher, read_feed_urls};
use tracing::{info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::SubscriberBuilder;

/// News ticker application that fetches and displays RSS feeds
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output in Waybar JSON format
    #[arg(long)]
    waybar: bool,

    /// Display only the link of the current entry
    #[arg(long)]
    link: bool,

    /// Advance to next entry without displaying
    #[arg(long)]
    next: bool,

    /// Go to previous entry without displaying (alias: --prev)
    #[arg(long, alias = "prev")]
    previous: bool,

    /// Refresh feed data from specified file
    #[arg(long, value_name = "FILE")]
    refresh: Option<String>,

    /// Show verbose/debug output (can be specifiedmultiple times for more verbosity)
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Purge/clear all entries from the database
    #[arg(long)]
    purge: bool,
}

#[tokio::main]
async fn main() {
    let verbose = std::env::args().any(|arg| arg == "-v" || arg == "--verbose");

    let builder = SubscriberBuilder::default();
    let builder = if verbose {
        builder.with_max_level(LevelFilter::DEBUG)
    } else {
        builder.with_max_level(LevelFilter::WARN)
    };

    builder.init();

    let args = Args::parse();
    let mut fetcher = Fetcher::new().await.unwrap();

    if args.purge {
        let count = purge_db(&fetcher.db).await.unwrap();
        info!("Purged {} entries from database", count);
        std::process::exit(0);
    }

    if let Some(feeds_file) = args.refresh {
        let urls = read_feed_urls(&feeds_file).expect("Failed to read feeds file");
        let new_inserts = fetcher.refresh_with_urls(urls).await.unwrap();
        info!("Refreshed feed data - added {} new entries", new_inserts);
        std::process::exit(0);
    }

    let current = get_current(&fetcher.db).await.unwrap();

    match current {
        Some(db_entry) => {
            if args.link {
                println!("{}", db_entry.link);
            } else if args.waybar {
                println!("{}", db_entry.display_waybar());
            } else if !args.next && !args.previous {
                println!("{}", db_entry.display());
            }

            if args.next {
                let _ = advance_to_next(&fetcher.db).await.unwrap();
            } else if args.previous {
                let _ = go_to_previous(&fetcher.db).await.unwrap();
            }
        }
        None => {
            if !args.next && !args.previous {
                warn!("No entries in database!");
            }
        }
    }
}
