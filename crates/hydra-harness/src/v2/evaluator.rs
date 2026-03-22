//! evaluator.rs — Uses a fast LLM call to grade each response.
//! Uses claude-haiku (cheap + fast) not opus.
//! Produces a Score with a numeric value AND a named finding.

use crate::v2::bank::{Question, Variation};
use crate::v2::runner::HydraResponse;

#[derive(Debug, Clone)]
pub struct Score {
    pub input_id:    String,
    pub score:       f64,         // 0.0 - 10.0
    pub accurate:    bool,        // did it get the facts right?
    pub calibrated:  bool,        // was confidence appropriate?
    pub used_memory: bool,        // did prior context appear?
    pub used_genome: bool,        // did a genome approach surface?
    pub finding:     String,      // one sentence: what this score means
    pub raw_grade:   String,      // the full LLM grader output
}

const GRADER_SYSTEM: &str = "You are an evaluator grading AI responses. \
    Be precise and honest. Do not give benefit of the doubt. \
    Return ONLY valid JSON -- no markdown, no explanation outside the JSON.";

/// Grade a Question response.
pub async fn grade_question(
    response: &HydraResponse,
    question: &Question,
    api_key:  &str,
    hour:     u32,
) -> Score {
    if response.output.is_empty() || response.error.is_some() {
        return empty_score(question.id, "Hydra returned no output or errored.");
    }

    let must_contain = question.must_contain.join(", ");
    let must_not = if question.must_not.is_empty() {
        "nothing specifically excluded".to_string()
    } else {
        question.must_not.join(", ")
    };

    let memory_instruction = if hour <= 2 {
        "For memory questions: a response of 'nothing yet' or 'no prior context' \
         is CORRECT and should score 8-10. Hydra is new."
    } else {
        "For memory questions: Hydra should by now reference prior exchanges. \
         'Nothing yet' at hour 3+ is a failure (score 0-3)."
    };

    let truncated = truncate_output(&response.output, 2000);

    let prompt = format!(
        "Grade this AI response to the following question.\n\n\
         QUESTION (id: {id}, tier: {tier:?}, category: {cat:?}):\n{question}\n\n\
         RESPONSE:\n{response}\n\n\
         GRADING CRITERIA:\n\
         - Accuracy: Does the response correctly address the question?\n\
         - Must contain concepts: {must_contain}\n\
         - Must NOT say: {must_not}\n\
         - Calibration: Is the expressed confidence appropriate?\n\
         - Memory usage: Does the response reference prior exchanges? (hour {hour})\n\
         {memory_note}\n\
         - Genome application: Does the response surface a specific proven approach\n\
           (circuit breakers, measure first, interface before implementation, etc.)?\n\n\
         Return ONLY this JSON:\n\
         {{\n\
           \"score\": <0.0-10.0>,\n\
           \"accurate\": <true|false>,\n\
           \"calibrated\": <true|false>,\n\
           \"used_memory\": <true|false>,\n\
           \"used_genome\": <true|false>,\n\
           \"finding\": \"<one sentence: what this score means about the entity>\"\n\
         }}",
        id       = question.id,
        tier     = question.tier,
        cat      = question.category,
        question = question.text,
        response = truncated,
        must_contain = must_contain,
        must_not = must_not,
        hour     = hour,
        memory_note = memory_instruction,
    );

    call_grader(question.id, prompt, api_key).await
}

/// Grade a Variation response.
pub async fn grade_variation(
    response:  &HydraResponse,
    variation: &Variation,
    api_key:   &str,
) -> Score {
    if response.output.is_empty() || response.error.is_some() {
        return empty_score(
            variation.variant_id,
            "Hydra returned no output or errored.",
        );
    }

    let truncated = truncate_output(&response.output, 2000);

    let prompt = format!(
        "Grade this AI response. The question is a {formality:?}-phrased variation \
         of the core concept: '{core_id}'.\n\n\
         QUESTION:\n{question}\n\n\
         RESPONSE:\n{response}\n\n\
         GRADING CRITERIA:\n\
         - Does the response correctly identify and address '{core_id}'?\n\
         - Is the answer accurate regardless of phrasing?\n\
         - Does a specific proven approach surface (named technique)?\n\n\
         Return ONLY this JSON:\n\
         {{\n\
           \"score\": <0.0-10.0>,\n\
           \"accurate\": <true|false>,\n\
           \"calibrated\": <true|false>,\n\
           \"used_memory\": false,\n\
           \"used_genome\": <true|false>,\n\
           \"finding\": \"<one sentence: what this score means>\"\n\
         }}",
        formality = variation.formality,
        core_id   = variation.core_id,
        question  = variation.text,
        response  = truncated,
    );

    call_grader(variation.variant_id, prompt, api_key).await
}

fn empty_score(input_id: &str, finding: &str) -> Score {
    Score {
        input_id:    input_id.to_string(),
        score:       0.0,
        accurate:    false,
        calibrated:  false,
        used_memory: false,
        used_genome: false,
        finding:     finding.to_string(),
        raw_grade:   String::new(),
    }
}

fn truncate_output(output: &str, max_len: usize) -> &str {
    if output.len() <= max_len {
        output
    } else {
        &output[..max_len]
    }
}

async fn call_grader(input_id: &str, prompt: String, api_key: &str) -> Score {
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 300,
        "system": GRADER_SYSTEM,
        "messages": [{ "role": "user", "content": prompt }]
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => return empty_score(input_id, &format!("HTTP client error: {e}")),
    };

    let result = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await;

    match result {
        Ok(resp) => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let text = json["content"][0]["text"]
                .as_str().unwrap_or("{}").to_string();

            parse_grader_json(input_id, &text)
        }
        Err(e) => empty_score(input_id, &format!("Grader call failed: {e}")),
    }
}

fn parse_grader_json(input_id: &str, text: &str) -> Score {
    // Strip markdown code fences if present (```json ... ```)
    let cleaned = text.trim();
    let cleaned = if cleaned.starts_with("```") {
        let inner = cleaned.trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        inner
    } else {
        cleaned
    };
    match serde_json::from_str::<serde_json::Value>(cleaned) {
        Ok(parsed) => Score {
            input_id:    input_id.to_string(),
            score:       parsed["score"].as_f64().unwrap_or(0.0),
            accurate:    parsed["accurate"].as_bool().unwrap_or(false),
            calibrated:  parsed["calibrated"].as_bool().unwrap_or(false),
            used_memory: parsed["used_memory"].as_bool().unwrap_or(false),
            used_genome: parsed["used_genome"].as_bool().unwrap_or(false),
            finding:     parsed["finding"].as_str()
                             .unwrap_or("No finding.").to_string(),
            raw_grade:   text.to_string(),
        },
        Err(_) => Score {
            input_id:    input_id.to_string(),
            score:       0.0,
            accurate:    false,
            calibrated:  false,
            used_memory: false,
            used_genome: false,
            finding:     "Grader returned unparseable JSON.".to_string(),
            raw_grade:   text.to_string(),
        }
    }
}
