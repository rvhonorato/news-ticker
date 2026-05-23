use feed_rs::model::Entry;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

/// A simplified entry from the database
#[derive(Debug, Clone)]
pub struct DbEntry {
    pub title: String,
    pub link: String,
    pub summary: String,
}

impl DbEntry {
    pub fn new(title: String, link: String, summary: String) -> Self {
        Self {
            title,
            link,
            summary,
        }
    }
}

impl DbEntry {
    pub fn display(&self) -> String {
        format!("📰 {}\n   {}\n   {}", self.title, self.link, self.summary)
    }

    pub fn display_waybar(&self) -> String {
        // Escape JSON special characters: backslash, double quote, control chars
        let escaped_title = self
            .title
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        format!(r#"{{"text":"{}","class":"feed"}}"#, escaped_title)
    }
}

pub async fn init_db() -> Result<Pool<Sqlite>, String> {
    let home_dir = std::env::var("HOME")
        .map_err(|e| format!("Failed to get HOME directory: {}", e))?;
    let cache_dir = std::path::Path::new(&home_dir).join(".cache/news-ticker");
    let db_path = cache_dir.join("db.sqlite");

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Use sqlite:// URL with mode=rwc (read, write, create) to ensure file is created
    let connection_string = format!("sqlite://{}?mode=rwc", db_path.display());

    let db = SqlitePoolOptions::new()
        .connect(&connection_string)
        .await
        .map_err(|e| {
            format!(
                "Failed to connect to database at '{}': {}",
                db_path.display(),
                e
            )
        })?;

    // Create table if not exists
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                link TEXT NOT NULL UNIQUE,
                summary TEXT,
                current BOOLEAN DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
    )
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to create table: {}", e))?;

    // Create unique index on link to prevent duplicates (in case table already existed without UNIQUE)
    let _ = sqlx::query(r#"CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_link ON entries(link)"#)
        .execute(&db)
        .await;

    Ok(db)
}

/// Adds an entry to the database. Returns true if inserted, false if duplicate was ignored.
pub async fn add_entry_to_db(db: &Pool<Sqlite>, entry: &Entry) -> Result<bool, String> {
    let title = entry
        .title
        .as_ref()
        .map(|t| t.content.clone())
        .unwrap_or_default();
    let link = entry
        .links
        .first()
        .map(|l| l.href.clone())
        .unwrap_or_default();
    let summary = entry
        .summary
        .as_ref()
        .map(|s| s.content.clone())
        .unwrap_or_default();

    // Use INSERT OR IGNORE to skip duplicates (based on unique link)
    let result = sqlx::query(
        r#"
                INSERT OR IGNORE INTO entries (title, link, summary)
                VALUES (?, ?, ?)
                "#,
    )
    .bind(title)
    .bind(link)
    .bind(summary)
    .execute(db)
    .await
    .map_err(|e| format!("Failed to insert entry: {}", e))?;

    // If rows_affected > 0, it was a new insert; if 0, it was a duplicate
    Ok(result.rows_affected() > 0)
}

/// Get the current entry (where current = 1), ordered by id
pub async fn get_current(db: &Pool<Sqlite>) -> Result<Option<DbEntry>, String> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        r#"SELECT title, link, summary FROM entries WHERE current = 1 ORDER BY id ASC LIMIT 1"#,
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Failed to fetch current entry: {}", e))?;

    Ok(row.map(|(title, link, summary)| DbEntry::new(title, link, summary)))
}

/// Set the next entry as current. Returns the new current entry as DbEntry.
/// If no current entry exists, sets the first entry (lowest id) as current.
/// If at the last entry, wraps around to the first.
pub async fn advance_to_next(db: &Pool<Sqlite>) -> Result<Option<DbEntry>, String> {
    // Get the next entry (by id), or wrap around to first if at the end
    let next_entry: Option<(i64, String, String, String)> = sqlx::query_as(
        r#"SELECT id, title, link, summary FROM entries 
           WHERE id > COALESCE((SELECT id FROM entries WHERE current = 1 LIMIT 1), 0) 
           ORDER BY id ASC LIMIT 1"#,
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Failed to fetch next entry: {}", e))?;

    match next_entry {
        Some((next_id, title, link, summary)) => {
            // Clear all current flags
            sqlx::query("UPDATE entries SET current = 0")
                .execute(db)
                .await
                .map_err(|e| format!("Failed to clear current: {}", e))?;

            // Set the next entry as current
            sqlx::query("UPDATE entries SET current = 1 WHERE id = ?")
                .bind(next_id)
                .execute(db)
                .await
                .map_err(|e| format!("Failed to set next current: {}", e))?;

            Ok(Some(DbEntry::new(title, link, summary)))
        }
        None => {
            // No next entry found, wrap around to first entry
            let first: Option<(i64, String, String, String)> = sqlx::query_as(
                r#"SELECT id, title, link, summary FROM entries ORDER BY id ASC LIMIT 1"#,
            )
            .fetch_optional(db)
            .await
            .map_err(|e| format!("Failed to fetch first entry: {}", e))?;

            match first {
                Some((first_id, title, link, summary)) => {
                    // Clear all current flags
                    sqlx::query("UPDATE entries SET current = 0")
                        .execute(db)
                        .await
                        .map_err(|e| format!("Failed to clear current: {}", e))?;

                    // Set the first entry as current
                    sqlx::query("UPDATE entries SET current = 1 WHERE id = ?")
                        .bind(first_id)
                        .execute(db)
                        .await
                        .map_err(|e| format!("Failed to set first as current: {}", e))?;

                    Ok(Some(DbEntry::new(title, link, summary)))
                }
                None => Ok(None), // No entries at all
            }
        }
    }
}

/// Initialize current entry if none exists (set first entry as current)
pub async fn init_current_entry(db: &Pool<Sqlite>) -> Result<(), String> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entries WHERE current = 1")
        .fetch_one(db)
        .await
        .map_err(|e| format!("Failed to count current entries: {}", e))?;

    if count == 0 {
        sqlx::query("UPDATE entries SET current = 0")
            .execute(db)
            .await
            .map_err(|e| format!("Failed to clear current: {}", e))?;

        sqlx::query("UPDATE entries SET current = 1 WHERE id = (SELECT MIN(id) FROM entries)")
            .execute(db)
            .await
            .map_err(|e| format!("Failed to set first entry as current: {}", e))?;
    }

    Ok(())
}

/// Go to the previous entry as current. Returns the new current entry as DbEntry.
/// If no current entry exists, sets the last entry (highest id) as current.
/// If at the first entry, wraps around to the last.
pub async fn go_to_previous(db: &Pool<Sqlite>) -> Result<Option<DbEntry>, String> {
    // Get the previous entry (by id), or wrap around to last if at the beginning
    let prev_entry: Option<(i64, String, String, String)> = sqlx::query_as(
        r#"SELECT id, title, link, summary FROM entries 
           WHERE id < COALESCE((SELECT id FROM entries WHERE current = 1 LIMIT 1), (SELECT MAX(id) + 1 FROM entries)) 
           ORDER BY id DESC LIMIT 1"#,
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Failed to fetch previous entry: {}", e))?;

    match prev_entry {
        Some((prev_id, title, link, summary)) => {
            // Clear all current flags
            sqlx::query("UPDATE entries SET current = 0")
                .execute(db)
                .await
                .map_err(|e| format!("Failed to clear current: {}", e))?;

            // Set the previous entry as current
            sqlx::query("UPDATE entries SET current = 1 WHERE id = ?")
                .bind(prev_id)
                .execute(db)
                .await
                .map_err(|e| format!("Failed to set previous current: {}", e))?;

            Ok(Some(DbEntry::new(title, link, summary)))
        }
        None => {
            // No previous entry found, wrap around to last entry
            let last: Option<(i64, String, String, String)> = sqlx::query_as(
                r#"SELECT id, title, link, summary FROM entries ORDER BY id DESC LIMIT 1"#,
            )
            .fetch_optional(db)
            .await
            .map_err(|e| format!("Failed to fetch last entry: {}", e))?;

            match last {
                Some((last_id, title, link, summary)) => {
                    // Clear all current flags
                    sqlx::query("UPDATE entries SET current = 0")
                        .execute(db)
                        .await
                        .map_err(|e| format!("Failed to clear current: {}", e))?;

                    // Set the last entry as current
                    sqlx::query("UPDATE entries SET current = 1 WHERE id = ?")
                        .bind(last_id)
                        .execute(db)
                        .await
                        .map_err(|e| format!("Failed to set last as current: {}", e))?;

                    Ok(Some(DbEntry::new(title, link, summary)))
                }
                None => Ok(None), // No entries at all
            }
        }
    }
}
