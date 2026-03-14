//! Response verification pipeline — extract claims and verify before delivery.
//! Phase 2: extracts factual claims from LLM responses, verifies against
//! sisters (Veritas, Codebase, Memory), corrects or flags unverified claims.

use std::sync::Arc;
use crate::sisters::Sisters;
use crate::cognitive::intent_router::{IntentCategory, ClassifiedIntent};

#[derive(Debug, Clone)]
pub(crate) struct VerificationResult {
    pub original_response: String,
    pub verified_response: String,
    pub claims_checked: usize,
    pub claims_verified: usize,
    pub claims_corrected: usize,
    pub claims_flagged: usize,
    pub verification_ms: u64,
}

#[derive(Debug, Clone)]
struct Claim { text: String, claim_type: ClaimType, span_start: usize, span_end: usize }

#[derive(Debug, Clone, PartialEq)]
enum ClaimType { FilePath, CodeSymbol, Quantitative, MemoryFact, ApiSyntax }

#[derive(Debug)]
enum ClaimStatus { Verified, Corrected(String), Flagged, Skipped }

/// Determines whether a response warrants claim verification.
///
/// Content-based: verifies ANY response that contains verifiable claims
/// (file paths, code symbols), regardless of intent category.
/// Only skips pure greetings/farewells with no technical content.
pub(crate) fn should_verify(
    intent: &ClassifiedIntent,
    _complexity: f32,
    response: &str,
) -> bool {
    // Never verify greeting/farewell unless they contain code references
    if matches!(intent.category,
        IntentCategory::Greeting | IntentCategory::Farewell | IntentCategory::Thanks
    ) {
        // Even greetings get verified if they mention file paths
        return has_verifiable_content(response);
    }
    // Verify any response with verifiable content (file paths, code symbols)
    has_verifiable_content(response) || response.len() >= 80
}

/// Check if response contains content worth verifying.
fn has_verifiable_content(response: &str) -> bool {
    // File path indicators
    let has_paths = response.contains(".rs")
        || response.contains(".ts")
        || response.contains(".py")
        || response.contains(".go")
        || response.contains(".js")
        || response.contains("src/")
        || response.contains("crates/");
    if has_paths { return true; }
    // Code symbol indicators
    let has_symbols = response.contains("fn ")
        || response.contains("struct ")
        || response.contains("class ")
        || response.contains("def ")
        || response.contains("func ");
    has_symbols
}

fn extract_claims(response: &str) -> Vec<Claim> {
    let mut claims = Vec::new();
    extract_file_paths(response, &mut claims);
    extract_code_symbols(response, &mut claims);
    extract_quantitative(response, &mut claims);
    extract_memory_facts(response, &mut claims);
    claims.sort_by_key(|c| c.span_start);
    dedup_overlapping(&mut claims);
    claims
}

fn extract_file_paths(response: &str, claims: &mut Vec<Claim>) {
    let bytes = response.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Start on /, ~, . OR alphanumeric preceded by whitespace/delimiter
        let is_start = (matches!(bytes[i], b'/' | b'~' | b'.')
            || (bytes[i].is_ascii_alphanumeric()))
            && (i == 0 || matches!(bytes[i - 1], b' ' | b'\t' | b'\n' | b'`' | b'"' | b'('));
        if !is_start { i += 1; continue; }
        let start = i;
        while i < bytes.len() && is_path_char(bytes[i]) { i += 1; }
        let candidate = &response[start..i];
        if candidate.len() > 4 && candidate.contains('/') {
            claims.push(Claim {
                text: candidate.to_string(),
                claim_type: ClaimType::FilePath,
                span_start: start,
                span_end: i,
            });
        }
    }
}

fn is_path_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'-' | b'_' | b'~')
}

fn extract_code_symbols(response: &str, claims: &mut Vec<Claim>) {
    let mut search_from = 0;
    while let Some(open) = response[search_from..].find('`') {
        let abs_open = search_from + open;
        let inner_start = abs_open + 1;
        if inner_start >= response.len() { break; }
        if let Some(close) = response[inner_start..].find('`') {
            let abs_close = inner_start + close;
            let symbol = &response[inner_start..abs_close];
            if symbol.len() >= 3
                && symbol.as_bytes()[0].is_ascii_alphabetic()
                && symbol.bytes().all(|b| b.is_ascii_alphanumeric()
                    || b == b'_' || b == b':')
                && !is_rust_keyword(symbol)
            {
                claims.push(Claim {
                    text: symbol.to_string(),
                    claim_type: ClaimType::CodeSymbol,
                    span_start: inner_start,
                    span_end: abs_close,
                });
            }
            search_from = abs_close + 1;
        } else {
            break;
        }
    }
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(s, "self" | "super" | "crate" | "pub" | "mod" | "use"
        | "let" | "mut" | "const" | "static" | "struct"
        | "enum" | "trait" | "impl" | "for" | "while" | "loop"
        | "match" | "return" | "async" | "await" | "true" | "false")
}

fn extract_quantitative(response: &str, claims: &mut Vec<Claim>) {
    let triggers = [
        "there are ", "contains ", "contain ", "has ", "have ",
        "found ", "includes ", "include ",
    ];
    let lower = response.to_lowercase();
    for trigger in &triggers {
        let mut offset = 0;
        while let Some(pos) = lower[offset..].find(trigger) {
            let abs = offset + pos;
            let after = abs + trigger.len();
            if after < response.len() && response.as_bytes()[after].is_ascii_digit() {
                let num_end = response[after..].find(|c: char| !c.is_ascii_digit())
                    .map(|i| after + i)
                    .unwrap_or(response.len());
                claims.push(Claim {
                    text: response[abs..num_end].to_string(),
                    claim_type: ClaimType::Quantitative,
                    span_start: abs,
                    span_end: num_end,
                });
            }
            offset = abs + trigger.len();
        }
    }
}

fn extract_memory_facts(response: &str, claims: &mut Vec<Claim>) {
    let triggers = ["you said", "you told me", "you mentioned", "you asked",
                     "last time", "previously", "earlier you"];
    let lower = response.to_lowercase();
    for trigger in &triggers {
        let mut offset = 0;
        while let Some(pos) = lower[offset..].find(trigger) {
            let abs = offset + pos;
            let end_limit = (abs + 120).min(response.len());
            let sentence_end = response[abs..end_limit]
                .find(['.', '\n', '!', '?'])
                .map(|i| abs + i)
                .unwrap_or(end_limit);
            claims.push(Claim {
                text: response[abs..sentence_end].to_string(),
                claim_type: ClaimType::MemoryFact,
                span_start: abs,
                span_end: sentence_end,
            });
            offset = sentence_end;
        }
    }
}

fn dedup_overlapping(claims: &mut Vec<Claim>) {
    let mut i = 0;
    while i + 1 < claims.len() {
        if claims[i].span_end > claims[i + 1].span_start {
            let len_a = claims[i].span_end - claims[i].span_start;
            let len_b = claims[i + 1].span_end - claims[i + 1].span_start;
            if len_a >= len_b { claims.remove(i + 1); } else { claims.remove(i); }
        } else {
            i += 1;
        }
    }
}

async fn verify_claim(claim: &Claim, sisters: &Arc<Sisters>) -> ClaimStatus {
    match claim.claim_type {
        ClaimType::FilePath => verify_file_path(&claim.text).await,
        ClaimType::CodeSymbol => verify_code_symbol(&claim.text, sisters).await,
        ClaimType::Quantitative => ClaimStatus::Skipped,
        ClaimType::MemoryFact => verify_memory_fact(&claim.text, sisters).await,
        ClaimType::ApiSyntax => ClaimStatus::Skipped,
    }
}

async fn verify_file_path(path: &str) -> ClaimStatus {
    let expanded = if path.starts_with('~') {
        match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            Ok(home) => path.replacen('~', &home, 1),
            Err(_) => return ClaimStatus::Skipped,
        }
    } else {
        path.to_string()
    };
    match tokio::fs::metadata(&expanded).await {
        Ok(_) => ClaimStatus::Verified,
        Err(_) => ClaimStatus::Flagged,
    }
}

async fn verify_code_symbol(symbol: &str, sisters: &Arc<Sisters>) -> ClaimStatus {
    match sisters.codebase_hallucination_check(symbol).await {
        Some(result) if result.contains("not found") => ClaimStatus::Flagged,
        Some(result) if result.contains("corrected") => {
            ClaimStatus::Corrected(result)
        }
        Some(_) => ClaimStatus::Verified,
        None => ClaimStatus::Skipped,
    }
}

async fn verify_memory_fact(fact: &str, sisters: &Arc<Sisters>) -> ClaimStatus {
    let query: String = fact.chars().take(200).collect();
    match sisters.memory_causal_query(&query).await {
        Some(result) if result.is_empty() => ClaimStatus::Flagged,
        Some(_) => ClaimStatus::Verified,
        None => ClaimStatus::Skipped,
    }
}

fn apply_corrections(response: &str, claims: &[Claim], statuses: &[ClaimStatus]) -> String {
    let mut result = response.to_string();
    for (claim, status) in claims.iter().zip(statuses.iter()).rev() {
        match status {
            ClaimStatus::Corrected(correction) => {
                let tag = format!("{} [corrected: {}]", claim.text, correction);
                result.replace_range(claim.span_start..claim.span_end, &tag);
            }
            ClaimStatus::Flagged => {
                let tag = format!("{} [unverified]", claim.text);
                result.replace_range(claim.span_start..claim.span_end, &tag);
            }
            _ => {}
        }
    }
    result
}

/// Verify an LLM response by extracting and checking factual claims.
pub(crate) async fn verify_response(
    response: &str,
    _user_text: &str,
    sisters: &Arc<Sisters>,
    intent: &ClassifiedIntent,
) -> VerificationResult {
    let start = std::time::Instant::now();
    let empty = || VerificationResult {
        original_response: response.to_string(),
        verified_response: response.to_string(),
        claims_checked: 0, claims_verified: 0,
        claims_corrected: 0, claims_flagged: 0,
        verification_ms: start.elapsed().as_millis() as u64,
    };

    if !should_verify(intent, 0.0, response) { return empty(); }
    let claims = extract_claims(response);
    if claims.is_empty() { return empty(); }

    // Verify all claims concurrently
    let mut statuses = Vec::with_capacity(claims.len());
    for claim in &claims {
        statuses.push(verify_claim(claim, sisters).await);
    }

    let (mut verified, mut corrected, mut flagged) = (0, 0, 0);
    for s in &statuses {
        match s {
            ClaimStatus::Verified => verified += 1,
            ClaimStatus::Corrected(_) => corrected += 1,
            ClaimStatus::Flagged => flagged += 1,
            ClaimStatus::Skipped => {}
        }
    }

    let verified_response = apply_corrections(response, &claims, &statuses);
    VerificationResult {
        original_response: response.to_string(),
        verified_response,
        claims_checked: claims.len(),
        claims_verified: verified,
        claims_corrected: corrected,
        claims_flagged: flagged,
        verification_ms: start.elapsed().as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent {
            category: cat,
            confidence: 0.9,
            target: None,
            payload: None,
        }
    }

    #[test]
    fn test_should_verify_code_intent() {
        let long = "x".repeat(100);
        assert!(should_verify(&make_intent(IntentCategory::CodeExplain), 0.5, &long));
        assert!(should_verify(&make_intent(IntentCategory::Question), 0.5, &long));
        assert!(should_verify(&make_intent(IntentCategory::CodeFix), 0.5, &long));
    }

    #[test]
    fn test_no_verify_greeting() {
        let long = "x".repeat(100);
        assert!(!should_verify(&make_intent(IntentCategory::Greeting), 0.5, &long));
        assert!(!should_verify(&make_intent(IntentCategory::Farewell), 0.5, &long));
        assert!(!should_verify(&make_intent(IntentCategory::Thanks), 0.1, &long));
    }

    #[test]
    fn test_extract_file_path_claims() {
        let resp = "The main entry point is src/main.rs and config at ~/.config/hydra end";
        let claims = extract_claims(resp);
        let paths: Vec<_> = claims.iter()
            .filter(|c| c.claim_type == ClaimType::FilePath)
            .collect();
        assert!(!paths.is_empty(), "should extract file paths");
        assert!(paths.iter().any(|c| c.text.contains("src/main.rs")));
    }

    #[test]
    fn test_extract_code_symbol_claims() {
        let resp = "Call `verify_response` to check, and `Sisters` handles dispatch.";
        let claims = extract_claims(resp);
        let syms: Vec<_> = claims.iter()
            .filter(|c| c.claim_type == ClaimType::CodeSymbol)
            .collect();
        assert!(syms.iter().any(|c| c.text == "verify_response"));
        assert!(syms.iter().any(|c| c.text == "Sisters"));
    }

    #[test]
    fn test_extract_quantitative_claims() {
        let resp = "The project contains 14 crates and there are 200 tests total.";
        let claims = extract_claims(resp);
        let quants: Vec<_> = claims.iter()
            .filter(|c| c.claim_type == ClaimType::Quantitative)
            .collect();
        assert_eq!(quants.len(), 2, "should find two quantitative claims");
    }

    #[test]
    fn test_apply_corrections() {
        let resp = "Check src/lib.rs for details";
        let claims = vec![Claim {
            text: "src/lib.rs".to_string(),
            claim_type: ClaimType::FilePath,
            span_start: 6,
            span_end: 16,
        }];
        let statuses = vec![ClaimStatus::Flagged];
        let result = apply_corrections(resp, &claims, &statuses);
        assert!(result.contains("[unverified]"));
        assert!(result.contains("src/lib.rs"));
    }
}
