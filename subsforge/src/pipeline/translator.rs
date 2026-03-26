use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::config::TranslatorConfig;
use crate::error::{SubsForgeError, Result};

#[derive(Serialize)]
struct TranslateRequest {
    text: Vec<String>,
    source_lang: String,
    target_lang: String,
}

#[derive(Deserialize)]
struct TranslateResponse {
    translations: Vec<String>,
}

/// NLLB-200 language codes (BCP-47 script format)
pub fn to_nllb_code(iso_code: &str) -> &str {
    match iso_code {
        "en" => "eng_Latn",
        "fr" => "fra_Latn",
        "ja" => "jpn_Jpan",
        "ko" => "kor_Hang",
        "de" => "deu_Latn",
        "es" => "spa_Latn",
        "it" => "ita_Latn",
        "pt" => "por_Latn",
        "zh" => "zho_Hans",
        "ru" => "rus_Cyrl",
        "ar" => "arb_Arab",
        "hi" => "hin_Deva",
        "th" => "tha_Thai",
        "vi" => "vie_Latn",
        "tr" => "tur_Latn",
        "pl" => "pol_Latn",
        "nl" => "nld_Latn",
        "sv" => "swe_Latn",
        "da" => "dan_Latn",
        "no" => "nob_Latn",
        other => other, // pass through if already NLLB format
    }
}

/// Translate a batch of texts using the NLLB-200 translation server.
pub async fn translate(
    config: &TranslatorConfig,
    client: &Client,
    texts: &[String],
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>> {
    let src = to_nllb_code(source_lang);
    let tgt = to_nllb_code(target_lang);

    info!(
        count = texts.len(),
        from = src,
        to = tgt,
        "translating batch"
    );

    let mut last_error = None;

    for attempt in 0..config.max_retries {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
        }

        let result = client
            .post(format!("{}/translate", config.endpoint))
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .json(&TranslateRequest {
                text: texts.to_vec(),
                source_lang: src.to_string(),
                target_lang: tgt.to_string(),
            })
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                let body: TranslateResponse = resp.json().await
                    .map_err(|e| SubsForgeError::Translation(format!("invalid response: {e}")))?;

                if body.translations.len() != texts.len() {
                    last_error = Some(format!(
                        "translation count mismatch: sent {}, got {}",
                        texts.len(),
                        body.translations.len()
                    ));
                    continue;
                }

                return Ok(body.translations);
            }
            Ok(resp) => {
                last_error = Some(format!("HTTP {}", resp.status()));
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    Err(SubsForgeError::Translation(
        last_error.unwrap_or_else(|| "unknown error".into())
    ))
}

/// Check if the translation server is healthy.
pub async fn health_check(config: &TranslatorConfig, client: &Client) -> bool {
    client
        .get(format!("{}/health", config.endpoint))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
}
