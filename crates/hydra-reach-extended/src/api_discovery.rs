//! ApiDiscovery — checks if a service has an API before resorting to browser automation.
//! Decision tree: API found → use executor HTTP call. No API → fall back to browser.

use serde::{Deserialize, Serialize};

/// Result of API discovery for a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDiscoveryResult {
    pub domain: String,
    pub has_api: bool,
    pub spec_url: Option<String>,
    pub api_type: ApiType,
    pub base_url: Option<String>,
}

/// Type of API discovered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiType {
    OpenApi,
    GraphQL,
    RestJson,
    Swagger,
    None,
}

/// Well-known API endpoints to probe.
const PROBE_PATHS: &[(&str, ApiType)] = &[
    ("/api", ApiType::RestJson),
    ("/api/v1", ApiType::RestJson),
    ("/api/v2", ApiType::RestJson),
    ("/.well-known/openapi", ApiType::OpenApi),
    ("/openapi.json", ApiType::OpenApi),
    ("/openapi.yaml", ApiType::OpenApi),
    ("/swagger.json", ApiType::Swagger),
    ("/swagger/v1/swagger.json", ApiType::Swagger),
    ("/graphql", ApiType::GraphQL),
    ("/api/graphql", ApiType::GraphQL),
];

/// Checks for API availability before browser fallback.
pub struct ApiDiscovery;

impl ApiDiscovery {
    /// Probe a domain for available APIs.
    /// Returns synchronously using blocking HTTP (not in the critical path).
    pub fn check_for_api(domain: &str) -> ApiDiscoveryResult {
        let base = if domain.starts_with("http") {
            domain.to_string()
        } else {
            format!("https://{domain}")
        };

        for (path, api_type) in PROBE_PATHS {
            let url = format!("{base}{path}");
            if Self::probe_url(&url) {
                eprintln!("hydra-reach: API found at {url} ({api_type:?})");
                return ApiDiscoveryResult {
                    domain: domain.into(),
                    has_api: true,
                    spec_url: Some(url),
                    api_type: api_type.clone(),
                    base_url: Some(base),
                };
            }
        }

        eprintln!("hydra-reach: no API found for {domain}, browser fallback needed");
        ApiDiscoveryResult {
            domain: domain.into(),
            has_api: false,
            spec_url: None,
            api_type: ApiType::None,
            base_url: None,
        }
    }

    /// Quick HEAD request to check if a URL responds.
    fn probe_url(url: &str) -> bool {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        match client.head(url).send() {
            Ok(resp) => {
                let status = resp.status();
                // 200-299 or 401/403 (API exists but needs auth)
                status.is_success() || status.as_u16() == 401 || status.as_u16() == 403
            }
            Err(_) => false,
        }
    }

    /// Decision: should we use API or browser for this domain?
    pub fn recommend(result: &ApiDiscoveryResult) -> ApiRecommendation {
        if result.has_api {
            match result.api_type {
                ApiType::GraphQL => ApiRecommendation::UseApi {
                    method: "POST".into(),
                    content_type: "application/json".into(),
                },
                ApiType::OpenApi | ApiType::Swagger | ApiType::RestJson => {
                    ApiRecommendation::UseApi {
                        method: "GET".into(),
                        content_type: "application/json".into(),
                    }
                }
                ApiType::None => ApiRecommendation::UseBrowser,
            }
        } else {
            ApiRecommendation::UseBrowser
        }
    }
}

/// Recommendation on how to interact with a service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiRecommendation {
    /// Use API calls (faster, more reliable).
    UseApi { method: String, content_type: String },
    /// Use browser automation (no API available).
    UseBrowser,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_api_for_unknown_domain() {
        let result = ApiDiscovery::check_for_api("hydra-nonexistent-test-12345.invalid");
        assert!(!result.has_api);
        assert_eq!(result.api_type, ApiType::None);
    }

    #[test]
    fn recommend_browser_for_no_api() {
        let result = ApiDiscoveryResult {
            domain: "test.com".into(),
            has_api: false,
            spec_url: None,
            api_type: ApiType::None,
            base_url: None,
        };
        assert_eq!(ApiDiscovery::recommend(&result), ApiRecommendation::UseBrowser);
    }

    #[test]
    fn recommend_api_for_graphql() {
        let result = ApiDiscoveryResult {
            domain: "api.example.com".into(),
            has_api: true,
            spec_url: Some("https://api.example.com/graphql".into()),
            api_type: ApiType::GraphQL,
            base_url: Some("https://api.example.com".into()),
        };
        let rec = ApiDiscovery::recommend(&result);
        assert!(matches!(rec, ApiRecommendation::UseApi { method, .. } if method == "POST"));
    }

    #[test]
    fn recommend_api_for_rest() {
        let result = ApiDiscoveryResult {
            domain: "example.com".into(),
            has_api: true,
            spec_url: Some("https://example.com/api".into()),
            api_type: ApiType::RestJson,
            base_url: Some("https://example.com".into()),
        };
        let rec = ApiDiscovery::recommend(&result);
        assert!(matches!(rec, ApiRecommendation::UseApi { method, .. } if method == "GET"));
    }

    #[test]
    fn api_discovery_result_serialization() {
        let result = ApiDiscoveryResult {
            domain: "test.com".into(),
            has_api: true,
            spec_url: Some("/api".into()),
            api_type: ApiType::OpenApi,
            base_url: Some("https://test.com".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: ApiDiscoveryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.api_type, ApiType::OpenApi);
    }
}
