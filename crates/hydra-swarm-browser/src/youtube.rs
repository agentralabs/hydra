//! YouTube transcript extraction — zero API keys, pure DOM scraping.
//! Navigates to video, clicks "Show transcript", extracts caption segments.

use crate::types::*;

/// Extract transcript from a YouTube video via browser DOM scraping.
pub async fn extract_transcript(
    engine: &mut hydra_browser::BrowserEngine,
    video_url: &str,
) -> Result<VideoTranscript, String> {
    // 1. Navigate to video
    engine.navigate(video_url).await
        .map_err(|e| format!("Navigation failed: {e}"))?;

    // 2. Wait for page to load fully
    engine.execute(&hydra_browser::BrowserAction::Wait {
        ms: crate::constants::YOUTUBE_TRANSCRIPT_WAIT_MS,
    }).await;

    // 3. Extract video title
    let title = extract_title(engine).await;

    // 4. Try to open transcript panel
    let transcript_opened = try_open_transcript(engine).await;

    if transcript_opened {
        // 5. Wait for transcript to render
        engine.execute(&hydra_browser::BrowserAction::Wait { ms: 2000 }).await;

        // 6. Extract transcript segments from DOM
        if let Some(segments) = scrape_transcript(engine).await {
            let full_text = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
            return Ok(VideoTranscript { video_url: video_url.into(), title, segments, full_text });
        }
    }

    // 7. Fallback: extract description + visible text
    let description = extract_description(engine).await;
    Ok(VideoTranscript {
        video_url: video_url.into(),
        title,
        segments: vec![TranscriptSegment { timestamp_secs: 0.0, text: description.clone() }],
        full_text: description,
    })
}

async fn extract_title(engine: &mut hydra_browser::BrowserEngine) -> String {
    let html = engine.html().await.unwrap_or_default();
    // Try <title> tag
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<title>") {
        let after = &html[start + 7..];
        if let Some(end) = after.to_lowercase().find("</title>") {
            let title = after[..end].trim().to_string();
            // Strip " - YouTube" suffix
            return title.replace(" - YouTube", "").trim().to_string();
        }
    }
    "Unknown Video".into()
}

async fn try_open_transcript(engine: &mut hydra_browser::BrowserEngine) -> bool {
    // Try multiple known selectors for the transcript button
    let selectors = [
        "button[aria-label=\"Show transcript\"]",
        "button[aria-label=\"Open transcript\"]",
        "#primary-button ytd-button-renderer",
        "tp-yt-paper-button[aria-label*=\"transcript\"]",
    ];

    for selector in &selectors {
        let result = engine.execute(&hydra_browser::BrowserAction::Click {
            selector: selector.to_string(),
        }).await;
        if result.success {
            eprintln!("hydra-youtube: opened transcript via {selector}");
            return true;
        }
    }

    // Try semantic nav as fallback
    let update = |_: u32, _: &str, _: &str, _: bool| {};
    match hydra_semantic_nav::try_semantic_nav_with_url(
        engine, "open transcript", "youtube.com", &update
    ).await {
        hydra_semantic_nav::NavResult::Success => {
            eprintln!("hydra-youtube: opened transcript via semantic nav");
            true
        }
        _ => {
            eprintln!("hydra-youtube: transcript button not found, using fallback");
            false
        }
    }
}

async fn scrape_transcript(engine: &mut hydra_browser::BrowserEngine) -> Option<Vec<TranscriptSegment>> {
    let html = engine.html().await.ok()?;
    parse_transcript_from_html(&html)
}

fn parse_transcript_from_html(html: &str) -> Option<Vec<TranscriptSegment>> {
    let mut segments = Vec::new();
    let lower = html.to_lowercase();

    // Look for transcript segment patterns in the HTML
    for chunk in lower.split("segment-timestamp") {
        if segments.len() > 500 { break; }
        // Extract timestamp
        let timestamp = extract_between(chunk, ">", "<")
            .map(|t| parse_timestamp(t.trim()))
            .unwrap_or(0.0);

        // Look for the text after the timestamp
        if let Some(text_start) = chunk.find("segment-text") {
            if let Some(text) = extract_between(&chunk[text_start..], ">", "<") {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    segments.push(TranscriptSegment { timestamp_secs: timestamp, text });
                }
            }
        }
    }

    if segments.is_empty() { None } else { Some(segments) }
}

fn parse_timestamp(ts: &str) -> f64 {
    let parts: Vec<&str> = ts.split(':').collect();
    match parts.len() {
        2 => {
            let mins: f64 = parts[0].parse().unwrap_or(0.0);
            let secs: f64 = parts[1].parse().unwrap_or(0.0);
            mins * 60.0 + secs
        }
        3 => {
            let hrs: f64 = parts[0].parse().unwrap_or(0.0);
            let mins: f64 = parts[1].parse().unwrap_or(0.0);
            let secs: f64 = parts[2].parse().unwrap_or(0.0);
            hrs * 3600.0 + mins * 60.0 + secs
        }
        _ => 0.0,
    }
}

async fn extract_description(engine: &mut hydra_browser::BrowserEngine) -> String {
    let result = engine.execute(&hydra_browser::BrowserAction::GetText).await;
    if result.success {
        // Truncate to first 2000 chars of visible text
        let text = &result.data;
        if text.len() > 2000 { text[..2000].to_string() } else { text.clone() }
    } else {
        "Could not extract video content".into()
    }
}

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let e = text[s..].find(end)? + s;
    Some(&text[s..e])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamp_mm_ss() {
        assert!((parse_timestamp("1:30") - 90.0).abs() < 0.1);
        assert!((parse_timestamp("0:05") - 5.0).abs() < 0.1);
    }

    #[test]
    fn parse_timestamp_hh_mm_ss() {
        assert!((parse_timestamp("1:30:00") - 5400.0).abs() < 0.1);
    }

    #[test]
    fn parse_transcript_html() {
        let html = r#"
            <div class="segment-timestamp">0:05</div>
            <div class="segment-text">Hello world</div>
            <div class="segment-timestamp">0:10</div>
            <div class="segment-text">Welcome to the video</div>
        "#;
        let segments = parse_transcript_from_html(html);
        assert!(segments.is_some());
        let segs = segments.unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "hello world");
    }
}
