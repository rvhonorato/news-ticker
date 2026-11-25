use feed_rs::parser;
use news_ticker::db::{
    DbEntry, add_entry_to_db, advance_to_next, entry_exists, get_current, go_to_previous,
    init_current_entry, purge_db,
};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn create_test_db() -> Pool<Sqlite> {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_path = format!("/tmp/news_ticker_test_{}.db", counter);

    // Clean up any existing test database
    let _ = fs::remove_file(&db_path);

    let connection_string = format!("sqlite://{}?mode=rwc", db_path);

    let db = SqlitePoolOptions::new()
        .connect(&connection_string)
        .await
        .unwrap();

    // Create table
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                link TEXT NOT NULL UNIQUE,
                summary TEXT,
                current BOOLEAN DEFAULT 0,
                is_offensive BOOLEAN DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
    )
    .execute(&db)
    .await
    .unwrap();

    // Create unique index on link
    sqlx::query(r#"CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_link ON entries(link)"#)
        .execute(&db)
        .await
        .unwrap();

    db
}

async fn add_test_entry(db: &Pool<Sqlite>, title: &str, link: &str, summary: &str) {
    sqlx::query("INSERT OR IGNORE INTO entries (title, link, summary) VALUES (?, ?, ?)")
        .bind(title)
        .bind(link)
        .bind(summary)
        .execute(db)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_db_init() {
    let db = create_test_db().await;
    // Verify we can query the database
    let result: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entries")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(result, 0);
}

#[tokio::test]
async fn test_add_and_get_entry() {
    let db = create_test_db().await;

    add_test_entry(&db, "Test Title", "https://test.com", "Test Summary").await;

    // Initialize current entry
    init_current_entry(&db).await.unwrap();

    let current = get_current(&db).await.unwrap();
    assert!(current.is_some());

    let entry = current.unwrap();
    assert_eq!(entry.title, "Test Title");
    assert_eq!(entry.link, "https://test.com");
    assert_eq!(entry.summary, "Test Summary");
}

#[tokio::test]
async fn test_navigation() {
    let db = create_test_db().await;

    // Add multiple entries
    for i in 0..3 {
        add_test_entry(
            &db,
            &format!("Entry {}", i),
            &format!("https://test{}.com", i),
            &format!("Summary {}", i),
        )
        .await;
    }

    // Initialize current entry (should be first entry)
    init_current_entry(&db).await.unwrap();

    let current = get_current(&db).await.unwrap();
    assert!(current.is_some());
    assert_eq!(current.unwrap().title, "Entry 0");

    // Navigate to next
    let next = advance_to_next(&db).await.unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().title, "Entry 1");

    // Navigate to previous
    let prev = go_to_previous(&db).await.unwrap();
    assert!(prev.is_some());
    assert_eq!(prev.unwrap().title, "Entry 0");
}

#[tokio::test]
async fn test_navigation_wraparound() {
    let db = create_test_db().await;

    // Add multiple entries
    for i in 0..3 {
        add_test_entry(
            &db,
            &format!("Entry {}", i),
            &format!("https://test{}.com", i),
            &format!("Summary {}", i),
        )
        .await;
    }

    init_current_entry(&db).await.unwrap();

    // Navigate to last entry by going next twice
    let _ = advance_to_next(&db).await.unwrap();
    let _ = advance_to_next(&db).await.unwrap();

    // Next should wrap around to first
    let next = advance_to_next(&db).await.unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().title, "Entry 0");

    // Previous from first should wrap to last
    let prev = go_to_previous(&db).await.unwrap();
    assert!(prev.is_some());
    assert_eq!(prev.unwrap().title, "Entry 2");
}

#[test]
fn test_db_entry_display() {
    let entry = DbEntry::new(
        "Test Title".to_string(),
        "https://test.com".to_string(),
        "Test Summary".to_string(),
        false,
    );

    let display = entry.display();
    assert!(display.contains("Test Title"));
    assert!(display.contains("https://test.com"));
    assert!(display.contains("Test Summary"));
    assert!(display.contains("📰"));
}

#[test]
fn test_db_entry_waybar_display() {
    let entry = DbEntry::new(
        "Test Title".to_string(),
        "https://test.com".to_string(),
        "Test Summary".to_string(),
        false,
    );

    let display = entry.display_waybar();
    assert!(display.contains("\"text\""));
    assert!(display.contains("\"class\":\"feed\""));
    assert!(display.contains("Test Title"));
}

#[tokio::test]
async fn test_purge_db() {
    let db = create_test_db().await;

    // Add some entries
    for i in 0..3 {
        add_test_entry(
            &db,
            &format!("Entry {}", i),
            &format!("https://test{}.com", i),
            &format!("Summary {}", i),
        )
        .await;
    }

    // Verify entries exist
    let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entries")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count_before, 3);

    // Purge the database
    let purged_count = purge_db(&db).await.unwrap();
    assert_eq!(purged_count, 3);

    // Verify database is empty
    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entries")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count_after, 0);

    // Verify get_current returns None
    let current = get_current(&db).await.unwrap();
    assert!(current.is_none());
}

const RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Example RSS</title>
    <link>https://example.com/</link>
    <description>Test feed</description>
    <item>
      <title>First item</title>
      <link>https://example.com/rss/1</link>
      <description>Summary 1</description>
    </item>
    <item>
      <title>Second item</title>
      <link>https://example.com/rss/2</link>
      <description>Summary 2</description>
    </item>
  </channel>
</rss>"#;

async fn count_entries(db: &Pool<Sqlite>) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM entries")
        .fetch_one(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_add_entry_stores_offensive_flag() {
    let db = create_test_db().await;
    let entries = parser::parse(RSS.as_bytes()).unwrap().entries;

    assert!(add_entry_to_db(&db, &entries[0], true).await.unwrap());
    assert!(add_entry_to_db(&db, &entries[1], false).await.unwrap());
    // Same link again is ignored
    assert!(!add_entry_to_db(&db, &entries[0], false).await.unwrap());
    assert_eq!(count_entries(&db).await, 2);

    init_current_entry(&db).await.unwrap();
    let first = get_current(&db).await.unwrap().unwrap();
    assert!(first.is_offensive);
    let second = advance_to_next(&db).await.unwrap().unwrap();
    assert!(!second.is_offensive);
}

#[tokio::test]
async fn test_entry_exists() {
    let db = create_test_db().await;
    let entries = parser::parse(RSS.as_bytes()).unwrap().entries;

    assert!(!entry_exists(&db, &entries[0]).await.unwrap());
    add_entry_to_db(&db, &entries[0], false).await.unwrap();
    assert!(entry_exists(&db, &entries[0]).await.unwrap());
    assert!(!entry_exists(&db, &entries[1]).await.unwrap());
}
