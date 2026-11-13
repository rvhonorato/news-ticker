//! Guards against parser changes in feed-rs: db.rs reads title, first link and summary.

use feed_rs::parser;

const RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Example RSS</title>
    <link>https://example.com/</link>
    <description>Test feed</description>
    <item>
      <title>First RSS item</title>
      <link>https://example.com/rss/1</link>
      <description>Summary of the first item</description>
      <guid>https://example.com/rss/1</guid>
    </item>
    <item>
      <title>Second RSS item</title>
      <link>https://example.com/rss/2</link>
      <description>Summary of the second item</description>
    </item>
  </channel>
</rss>"#;

const ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Example Atom</title>
  <id>urn:example:atom</id>
  <updated>2026-01-01T00:00:00Z</updated>
  <entry>
    <title>First Atom entry</title>
    <id>urn:example:atom:1</id>
    <link href="https://example.com/atom/1"/>
    <updated>2026-01-01T00:00:00Z</updated>
    <summary>Summary of the Atom entry</summary>
  </entry>
</feed>"#;

fn fields(xml: &str) -> Vec<(String, String, String)> {
    parser::parse(xml.as_bytes())
        .unwrap()
        .entries
        .into_iter()
        .map(|e| {
            (
                e.title.map(|t| t.content).unwrap_or_default(),
                e.links.first().map(|l| l.href.clone()).unwrap_or_default(),
                e.summary.map(|s| s.content).unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn test_parse_rss() {
    let entries = fields(RSS);
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0],
        (
            "First RSS item".to_string(),
            "https://example.com/rss/1".to_string(),
            "Summary of the first item".to_string(),
        )
    );
    assert_eq!(entries[1].1, "https://example.com/rss/2");
}

#[test]
fn test_parse_atom() {
    let entries = fields(ATOM);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0],
        (
            "First Atom entry".to_string(),
            "https://example.com/atom/1".to_string(),
            "Summary of the Atom entry".to_string(),
        )
    );
}
