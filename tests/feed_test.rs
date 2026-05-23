use news_ticker::feed::read_feed_urls;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_read_feed_urls_valid() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "https://example.com/feed.xml").unwrap();
    writeln!(file, "https://another-feed.com/rss").unwrap();

    let path = file.path().to_str().unwrap();
    let urls = read_feed_urls(path).unwrap();

    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://example.com/feed.xml");
    assert_eq!(urls[1], "https://another-feed.com/rss");
}

#[test]
fn test_read_feed_urls_with_comments() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "# This is a comment").unwrap();
    writeln!(file, "https://valid-feed.com/rss").unwrap();
    writeln!(file, "# Another comment").unwrap();
    writeln!(file, "https://another-valid.com/feed").unwrap();

    let path = file.path().to_str().unwrap();
    let urls = read_feed_urls(path).unwrap();

    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://valid-feed.com/rss");
    assert_eq!(urls[1], "https://another-valid.com/feed");
}

#[test]
fn test_read_feed_urls_with_blanks() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "https://feed1.com/rss").unwrap();
    writeln!(file, "").unwrap();
    writeln!(file, "   ").unwrap();
    writeln!(file, "https://feed2.com/rss").unwrap();

    let path = file.path().to_str().unwrap();
    let urls = read_feed_urls(path).unwrap();

    assert_eq!(urls.len(), 2);
}

#[test]
fn test_read_feed_urls_empty_file() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    let result = read_feed_urls(path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No valid URLs"));
}

#[test]
fn test_read_feed_urls_nonexistent_file() {
    let result = read_feed_urls("/nonexistent/path/to/feeds.txt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Feeds file not found"));
}
