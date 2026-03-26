use sqlx::{Pool, Sqlite, SqlitePool, Row};

use crate::models::{Job, Translation, Stats};

pub async fn init_pool(url: &str) -> anyhow::Result<Pool<Sqlite>> {
    // Ensure parent directory exists
    if let Some(path) = url.strip_prefix("sqlite:") {
        let path = path.split('?').next().unwrap_or(path);
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = SqlitePool::connect(url).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    let sql = include_str!("../migrations/001_init.sql");
    sqlx::raw_sql(sql).execute(pool).await?;
    Ok(())
}

pub async fn create_job(
    pool: &Pool<Sqlite>,
    media_path: &str,
    source: &str,
    source_id: Option<i64>,
    title: Option<&str>,
    target_languages: &[String],
) -> anyhow::Result<i64> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO jobs (media_path, source, source_id, title) VALUES (?, ?, ?, ?)"
    )
    .bind(media_path)
    .bind(source)
    .bind(source_id)
    .bind(title)
    .execute(pool)
    .await?;

    let job_id = if result.rows_affected() == 0 {
        // Already exists
        let row = sqlx::query("SELECT id FROM jobs WHERE media_path = ?")
            .bind(media_path)
            .fetch_one(pool)
            .await?;
        row.get::<i64, _>("id")
    } else {
        result.last_insert_rowid()
    };

    for lang in target_languages {
        sqlx::query(
            "INSERT OR IGNORE INTO translations (job_id, target_language) VALUES (?, ?)"
        )
        .bind(job_id)
        .bind(lang)
        .execute(pool)
        .await?;
    }

    Ok(job_id)
}

pub async fn update_job_status(
    pool: &Pool<Sqlite>,
    job_id: i64,
    status: &str,
    failure_reason: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE jobs SET status = ?, failure_reason = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(status)
    .bind(failure_reason)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_job_language(
    pool: &Pool<Sqlite>,
    job_id: i64,
    language: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE jobs SET detected_language = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(language)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_translation_status(
    pool: &Pool<Sqlite>,
    translation_id: i64,
    status: &str,
    srt_path: Option<&str>,
    failure_reason: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE translations SET status = ?, srt_path = ?, failure_reason = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(status)
    .bind(srt_path)
    .bind(failure_reason)
    .bind(translation_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_pending_jobs(pool: &Pool<Sqlite>) -> anyhow::Result<Vec<Job>> {
    let jobs = sqlx::query_as::<_, Job>(
        "SELECT * FROM jobs WHERE status = 'pending' ORDER BY created_at"
    )
    .fetch_all(pool)
    .await?;
    Ok(jobs)
}

pub async fn get_jobs(pool: &Pool<Sqlite>, status: Option<&str>, limit: i64) -> anyhow::Result<Vec<Job>> {
    if let Some(status) = status {
        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE status = ? ORDER BY updated_at DESC LIMIT ?"
        )
        .bind(status)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(jobs)
    } else {
        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs ORDER BY updated_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(jobs)
    }
}

pub async fn get_job(pool: &Pool<Sqlite>, id: i64) -> anyhow::Result<Option<Job>> {
    let job = sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(job)
}

pub async fn get_translations_for_job(pool: &Pool<Sqlite>, job_id: i64) -> anyhow::Result<Vec<Translation>> {
    let translations = sqlx::query_as::<_, Translation>(
        "SELECT * FROM translations WHERE job_id = ?"
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;
    Ok(translations)
}

pub async fn retry_job(pool: &Pool<Sqlite>, job_id: i64) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE jobs SET status = 'pending', failure_reason = NULL, updated_at = datetime('now') WHERE id = ? AND status = 'failed'"
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE translations SET status = 'pending', failure_reason = NULL, updated_at = datetime('now') WHERE job_id = ? AND status = 'failed'"
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_stats(pool: &Pool<Sqlite>) -> anyhow::Result<Stats> {
    let row = sqlx::query(
        "SELECT
            COUNT(*) as total,
            SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
            SUM(CASE WHEN status NOT IN ('pending', 'completed', 'failed') THEN 1 ELSE 0 END) as in_progress
        FROM jobs"
    )
    .fetch_one(pool)
    .await?;

    Ok(Stats {
        total: row.get::<i64, _>("total"),
        pending: row.get::<i64, _>("pending"),
        completed: row.get::<i64, _>("completed"),
        failed: row.get::<i64, _>("failed"),
        in_progress: row.get::<i64, _>("in_progress"),
    })
}
