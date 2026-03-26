CREATE TABLE IF NOT EXISTS jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_path TEXT NOT NULL,
    source TEXT NOT NULL,
    source_id INTEGER,
    title TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    detected_language TEXT,
    failure_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(media_path)
);

CREATE TABLE IF NOT EXISTS translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL REFERENCES jobs(id),
    target_language TEXT NOT NULL,
    srt_path TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    failure_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(job_id, target_language)
);
