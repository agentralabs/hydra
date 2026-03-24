//! Single browser worker — navigates, extracts content, returns result.
//! Each worker gets its own BrowserEngine (Chrome process).

use crate::types::*;
use crate::youtube;
use tokio::sync::mpsc;

/// Run a single browser worker to completion.
pub async fn run_worker(
    task: SwarmTask,
    worker_id: uuid::Uuid,
    update_tx: mpsc::Sender<SwarmUpdate>,
) -> WorkerResult {
    let start = std::time::Instant::now();

    let _ = update_tx.send(SwarmUpdate::WorkerSpawned {
        worker_id, query: task.query.clone(),
    }).await;

    // Try Chrome first, fall back to HTTP-only
    let mut engine = hydra_browser::BrowserEngine::new();
    let has_chrome = engine.launch().await.is_ok();

    let result = if has_chrome {
        match &task.task_type {
            SwarmTaskType::WebSearch => {
                run_web_search(&mut engine, &task, worker_id, &update_tx).await
            }
            SwarmTaskType::DeepRead { url } => {
                run_deep_read(&mut engine, &task, url, worker_id, &update_tx).await
            }
            SwarmTaskType::YouTubeTranscript { video_url } => {
                run_youtube(&mut engine, &task, video_url, worker_id, &update_tx).await
            }
            SwarmTaskType::DocumentExtract { url } => {
                run_deep_read(&mut engine, &task, url, worker_id, &update_tx).await
            }
        }
    } else {
        run_http_fallback(&task, worker_id).await
    };

    if has_chrome { engine.close().await; }

    let duration_ms = start.elapsed().as_millis() as u64;
    let mut result = result;
    result.duration_ms = duration_ms;

    if result.error.is_some() {
        let _ = update_tx.send(SwarmUpdate::WorkerFailed {
            worker_id, error: result.error.clone().unwrap_or_default(),
        }).await;
    } else {
        let preview = result.content.chars().take(100).collect();
        let _ = update_tx.send(SwarmUpdate::WorkerComplete {
            worker_id, preview,
        }).await;
    }

    result
}

async fn run_web_search(
    engine: &mut hydra_browser::BrowserEngine,
    task: &SwarmTask,
    worker_id: uuid::Uuid,
    update_tx: &mpsc::Sender<SwarmUpdate>,
) -> WorkerResult {
    let _ = update_tx.send(SwarmUpdate::WorkerProgress {
        worker_id, status: format!("Searching: {}", task.query),
    }).await;

    // Use hydra-web search engine (async multi-engine fan-out)
    let mut orch = hydra_web::SearchOrchestrator::new();
    match orch.quick_search(&task.query).await {
        Ok(resp) => WorkerResult {
            task_id: task.id, worker_id,
            content: resp.format_display(),
            source_url: format!("search:{}", task.query),
            confidence: 0.7, duration_ms: resp.duration_ms, error: None,
        },
        Err(e) => WorkerResult {
            task_id: task.id, worker_id, content: String::new(),
            source_url: String::new(), confidence: 0.0, duration_ms: 0,
            error: Some(format!("{e}")),
        },
    }
}

async fn run_deep_read(
    engine: &mut hydra_browser::BrowserEngine,
    task: &SwarmTask,
    url: &str,
    worker_id: uuid::Uuid,
    update_tx: &mpsc::Sender<SwarmUpdate>,
) -> WorkerResult {
    let _ = update_tx.send(SwarmUpdate::WorkerProgress {
        worker_id, status: format!("Reading: {url}"),
    }).await;

    // Navigate
    if let Err(e) = engine.navigate(url).await {
        return WorkerResult {
            task_id: task.id, worker_id, content: String::new(),
            source_url: url.into(), confidence: 0.0, duration_ms: 0,
            error: Some(format!("Navigation: {e}")),
        };
    }

    // Extract content
    let html = engine.html().await.unwrap_or_default();
    let extracted = hydra_web::extractor::extract(&html);

    WorkerResult {
        task_id: task.id, worker_id,
        content: extracted.main_text,
        source_url: url.into(),
        confidence: if extracted.word_count > crate::constants::MIN_CONTENT_WORDS { 0.8 } else { 0.4 },
        duration_ms: 0, error: None,
    }
}

async fn run_youtube(
    engine: &mut hydra_browser::BrowserEngine,
    task: &SwarmTask,
    video_url: &str,
    worker_id: uuid::Uuid,
    update_tx: &mpsc::Sender<SwarmUpdate>,
) -> WorkerResult {
    let _ = update_tx.send(SwarmUpdate::WorkerProgress {
        worker_id, status: format!("Extracting transcript: {video_url}"),
    }).await;

    match youtube::extract_transcript(engine, video_url).await {
        Ok(transcript) => {
            let content = format!("# {}\n\n{}", transcript.title, transcript.full_text);
            WorkerResult {
                task_id: task.id, worker_id, content,
                source_url: video_url.into(),
                confidence: if transcript.segments.len() > 1 { 0.85 } else { 0.5 },
                duration_ms: 0, error: None,
            }
        }
        Err(e) => WorkerResult {
            task_id: task.id, worker_id, content: String::new(),
            source_url: video_url.into(), confidence: 0.0, duration_ms: 0,
            error: Some(e),
        },
    }
}

/// HTTP-only fallback when Chrome is unavailable.
async fn run_http_fallback(task: &SwarmTask, worker_id: uuid::Uuid) -> WorkerResult {
    let url = match &task.task_type {
        SwarmTaskType::DeepRead { url } | SwarmTaskType::DocumentExtract { url } => url.clone(),
        SwarmTaskType::YouTubeTranscript { video_url } => video_url.clone(),
        SwarmTaskType::WebSearch => {
            let mut orch = hydra_web::SearchOrchestrator::new();
            return match orch.quick_search(&task.query).await {
                Ok(resp) => WorkerResult {
                    task_id: task.id, worker_id, content: resp.format_display(),
                    source_url: format!("search:{}", task.query),
                    confidence: 0.6, duration_ms: 0, error: None,
                },
                Err(e) => WorkerResult {
                    task_id: task.id, worker_id, content: String::new(),
                    source_url: String::new(), confidence: 0.0, duration_ms: 0,
                    error: Some(format!("{e}")),
                },
            };
        }
    };

    // Simple HTTP fetch
    match reqwest::get(&url).await {
        Ok(resp) => {
            let html = resp.text().await.unwrap_or_default();
            let extracted = hydra_web::extractor::extract(&html);
            WorkerResult {
                task_id: task.id, worker_id, content: extracted.main_text,
                source_url: url, confidence: 0.5, duration_ms: 0, error: None,
            }
        }
        Err(e) => WorkerResult {
            task_id: task.id, worker_id, content: String::new(),
            source_url: url, confidence: 0.0, duration_ms: 0,
            error: Some(format!("HTTP: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_result_error_has_zero_confidence() {
        let r = WorkerResult {
            task_id: uuid::Uuid::new_v4(), worker_id: uuid::Uuid::new_v4(),
            content: String::new(), source_url: String::new(),
            confidence: 0.0, duration_ms: 0, error: Some("test".into()),
        };
        assert!(r.error.is_some());
        assert_eq!(r.confidence, 0.0);
    }
}
