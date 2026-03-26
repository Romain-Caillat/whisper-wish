pub mod ffmpeg;
pub mod srt;
pub mod translator;
pub mod whisper;

use std::path::Path;
use reqwest::Client;
use sqlx::{Pool, Sqlite};
use tempfile::NamedTempFile;
use tracing::{info, error};

use crate::config::Config;
use crate::db;
use crate::naming;

/// Process a single media file through the full pipeline:
/// extract audio → transcribe → translate → write SRT files.
pub async fn process_job(
    config: &Config,
    pool: &Pool<Sqlite>,
    client: &Client,
    job_id: i64,
    media_path: &Path,
) -> anyhow::Result<()> {
    // 1. Extract audio
    db::update_job_status(pool, job_id, "extracting", None).await?;
    let temp_wav = NamedTempFile::with_suffix(".wav")?;
    let wav_path = temp_wav.path().to_path_buf();

    if let Err(e) = ffmpeg::extract_audio(&config.ffmpeg, media_path, &wav_path).await {
        db::update_job_status(pool, job_id, "failed", Some(&e.to_string())).await?;
        return Err(e.into());
    }

    // 2. Transcribe
    db::update_job_status(pool, job_id, "transcribing", None).await?;
    let whisper_result = match whisper::transcribe(&config.whisper, &wav_path).await {
        Ok(r) => r,
        Err(e) => {
            db::update_job_status(pool, job_id, "failed", Some(&e.to_string())).await?;
            return Err(e.into());
        }
    };

    // Drop temp file
    drop(temp_wav);

    let detected_lang = whisper_result.detected_language.as_deref()
        .unwrap_or(&config.whisper.language);
    db::update_job_language(pool, job_id, detected_lang).await?;
    info!(language = detected_lang, "detected source language");

    // 3. Parse SRT
    let entries = match srt::parse(&whisper_result.srt_content) {
        Ok(e) => e,
        Err(e) => {
            db::update_job_status(pool, job_id, "failed", Some(&e.to_string())).await?;
            return Err(e.into());
        }
    };

    // Save original language SRT if configured
    if config.general.save_original_srt {
        let original_srt_path = naming::srt_path(media_path, detected_lang);
        tokio::fs::write(&original_srt_path, &whisper_result.srt_content).await?;
        info!(path = %original_srt_path.display(), "saved original SRT");
    }

    // 4. Translate to each target language
    db::update_job_status(pool, job_id, "translating", None).await?;
    let translations = db::get_translations_for_job(pool, job_id).await?;
    let source_texts = srt::extract_texts(&entries);

    for translation in &translations {
        let target_lang = &translation.target_language;

        if target_lang == detected_lang {
            // Source == target: just copy the original SRT
            let out_path = naming::srt_path(media_path, target_lang);
            tokio::fs::write(&out_path, &whisper_result.srt_content).await?;
            db::update_translation_status(
                pool,
                translation.id,
                "completed",
                Some(&out_path.to_string_lossy()),
                None,
            ).await?;
            info!(lang = target_lang, "copied original SRT (same language)");
            continue;
        }

        db::update_translation_status(pool, translation.id, "in_progress", None, None).await?;

        match translator::translate(
            &config.translator,
            client,
            &source_texts,
            detected_lang,
            target_lang,
        ).await {
            Ok(translated_texts) => {
                match srt::replace_texts(&entries, &translated_texts) {
                    Ok(translated_entries) => {
                        let out_path = naming::srt_path(media_path, target_lang);
                        let content = srt::serialize(&translated_entries);
                        tokio::fs::write(&out_path, &content).await?;
                        db::update_translation_status(
                            pool,
                            translation.id,
                            "completed",
                            Some(&out_path.to_string_lossy()),
                            None,
                        ).await?;
                        info!(lang = target_lang, path = %out_path.display(), "saved translated SRT");
                    }
                    Err(e) => {
                        error!(lang = target_lang, error = %e, "SRT reassembly failed");
                        db::update_translation_status(
                            pool, translation.id, "failed", None, Some(&e.to_string()),
                        ).await?;
                    }
                }
            }
            Err(e) => {
                error!(lang = target_lang, error = %e, "translation failed");
                db::update_translation_status(
                    pool, translation.id, "failed", None, Some(&e.to_string()),
                ).await?;
            }
        }
    }

    // Check if all translations completed
    let updated_translations = db::get_translations_for_job(pool, job_id).await?;
    let all_done = updated_translations.iter().all(|t| t.status == "completed");
    let any_failed = updated_translations.iter().any(|t| t.status == "failed");

    if all_done {
        db::update_job_status(pool, job_id, "completed", None).await?;
    } else if any_failed {
        db::update_job_status(pool, job_id, "failed", Some("one or more translations failed")).await?;
    }

    Ok(())
}
