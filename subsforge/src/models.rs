use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Job {
    pub id: i64,
    pub media_path: String,
    pub source: String,
    pub source_id: Option<i64>,
    pub title: Option<String>,
    pub status: String,
    pub detected_language: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Translation {
    pub id: i64,
    pub job_id: i64,
    pub target_language: String,
    pub srt_path: Option<String>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobWithTranslations {
    #[serde(flatten)]
    pub job: Job,
    pub translations: Vec<Translation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total: i64,
    pub pending: i64,
    pub completed: i64,
    pub failed: i64,
    pub in_progress: i64,
}
