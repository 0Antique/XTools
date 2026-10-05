pub const SCHEMA: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS applications (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, normalized_name TEXT NOT NULL,
    pinyin TEXT NOT NULL, pinyin_initials TEXT NOT NULL, source TEXT NOT NULL,
    launch_type TEXT NOT NULL, launch_target TEXT NOT NULL, arguments TEXT,
    working_directory TEXT, icon_path TEXT, unique_key TEXT NOT NULL UNIQUE,
    last_seen_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS app_usage (
    app_id TEXT PRIMARY KEY, launch_count INTEGER NOT NULL DEFAULT 0,
    last_launched_at INTEGER
);
CREATE TABLE IF NOT EXISTS clipboard_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT, item_type TEXT NOT NULL,
    text_content TEXT, data_path TEXT, preview TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL,
    is_favorite INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS clipboard_order ON clipboard_items(created_at DESC);
CREATE TABLE IF NOT EXISTS rename_batches (
    id INTEGER PRIMARY KEY AUTOINCREMENT, created_at INTEGER NOT NULL,
    undone INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS rename_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT, batch_id INTEGER NOT NULL,
    old_path TEXT NOT NULL, new_path TEXT NOT NULL,
    FOREIGN KEY(batch_id) REFERENCES rename_batches(id) ON DELETE CASCADE
);
PRAGMA user_version=1;
"#;
