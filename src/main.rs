use clap::Parser;
use news_ticker::db::{advance_to_next, get_current, go_to_previous, purge_db};
use news_ticker::feed::{Fetcher, read_feed_urls};

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

    /// Show verbose/debug output
    #[arg(long, short)]
    verbose: bool,

    /// Purge/clear all entries from the database
    #[arg(long)]
    purge: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut fetcher = Fetcher::new().await.unwrap();

    // Purge database if explicitly requested
    if args.purge {
        let count = purge_db(&fetcher.db).await.unwrap();
        eprintln!("Purged {} entries from database", count);

        std::process::exit(0);
    }

    // Refresh feed data if explicitly requested
    if let Some(feeds_file) = args.refresh {
        let urls = read_feed_urls(&feeds_file).expect("Failed to read feeds file");
        let new_inserts = fetcher.refresh_with_urls(urls).await.unwrap();
        eprintln!("Refreshed feed data - added {} new entries", new_inserts);

        std::process::exit(0);
    }

    // Get the current entry
    let current = get_current(&fetcher.db).await.unwrap();

    match current {
        Some(db_entry) => {
            // Handle display based on arguments
            if args.link {
                // Display only the link
                println!("{}", db_entry.link);
            } else if args.waybar {
                // Output Waybar JSON format
                println!("{}", db_entry.display_waybar());
            } else if !args.next && !args.previous {
                // Normal display
                println!("{}", db_entry.display());
            }

            // Handle navigation
            if args.next {
                let _ = advance_to_next(&fetcher.db).await.unwrap();
            } else if args.previous {
                let _ = go_to_previous(&fetcher.db).await.unwrap();
            }
        }
        None => {
            if !args.next && !args.previous {
                eprintln!("No entries in database!");
            }
        }
    }
}
