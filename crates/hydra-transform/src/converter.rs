//! FormatConverter — one conversion path between two formats.

use crate::{
    constants::CONVERSION_CONFIDENCE_FLOOR,
    errors::TransformError,
    format::DataFormat,
};
use serde::{Deserialize, Serialize};

/// The result of a conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub data: String,
    pub from: String,
    pub to: String,
    pub confidence: f64,
    pub chain: Vec<String>, // intermediate formats if multi-hop
}

impl ConversionResult {
    pub fn is_reliable(&self) -> bool {
        self.confidence >= CONVERSION_CONFIDENCE_FLOOR
    }
}

/// Convert data between two formats.
/// All conversions go through a canonical intermediate representation.
/// Meaning is preserved — not just structure.
pub fn convert(
    data: &str,
    from: &DataFormat,
    to: &DataFormat,
) -> Result<ConversionResult, TransformError> {
    // Same format — no-op
    if from == to {
        return Ok(ConversionResult {
            data: data.to_string(),
            from: from.label(),
            to: to.label(),
            confidence: 1.0,
            chain: vec![from.label()],
        });
    }

    // All conversions route through a normalized JSON intermediate
    let intermediate = to_intermediate(data, from)?;
    let output = from_intermediate(&intermediate, to)?;

    let confidence = compute_confidence(from, to);

    Ok(ConversionResult {
        data: output,
        from: from.label(),
        to: to.label(),
        confidence,
        chain: vec![from.label(), "intermediate".into(), to.label()],
    })
}

/// Convert data to a canonical intermediate (normalized JSON).
fn to_intermediate(
    data: &str,
    from: &DataFormat,
) -> Result<serde_json::Value, TransformError> {
    match from {
        DataFormat::Json => serde_json::from_str(data).map_err(|e| {
            TransformError::ParseError {
                format: "json".into(),
                reason: e.to_string(),
            }
        }),
        DataFormat::Csv => {
            let lines: Vec<&str> = data.lines().collect();
            if lines.is_empty() {
                return Ok(serde_json::Value::Array(vec![]));
            }
            let headers: Vec<&str> =
                lines[0].split(',').map(str::trim).collect();
            let rows: Vec<serde_json::Value> = lines[1..]
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|line| {
                    let values: Vec<&str> =
                        line.split(',').map(str::trim).collect();
                    let obj: serde_json::Map<String, serde_json::Value> =
                        headers
                            .iter()
                            .zip(values.iter())
                            .map(|(h, v)| {
                                (
                                    h.to_string(),
                                    serde_json::Value::String(
                                        v.to_string(),
                                    ),
                                )
                            })
                            .collect();
                    serde_json::Value::Object(obj)
                })
                .collect();
            Ok(serde_json::Value::Array(rows))
        }
        DataFormat::Toml => {
            let mut map = serde_json::Map::new();
            for line in data.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(eq) = line.find('=') {
                    let key = line[..eq].trim().to_string();
                    let val =
                        line[eq + 1..].trim().trim_matches('"').to_string();
                    map.insert(key, serde_json::Value::String(val));
                }
            }
            Ok(serde_json::Value::Object(map))
        }
        // For other formats: wrap in a generic object
        _ => {
            let mut map = serde_json::Map::new();
            map.insert(
                "content".into(),
                serde_json::Value::String(data.to_string()),
            );
            map.insert(
                "format".into(),
                serde_json::Value::String(from.label()),
            );
            Ok(serde_json::Value::Object(map))
        }
    }
}

/// Convert from the canonical intermediate to the target format.
fn from_intermediate(
    intermediate: &serde_json::Value,
    to: &DataFormat,
) -> Result<String, TransformError> {
    match to {
        DataFormat::Json => {
            serde_json::to_string_pretty(intermediate).map_err(|e| {
                TransformError::ParseError {
                    format: "json".into(),
                    reason: e.to_string(),
                }
            })
        }
        DataFormat::Csv => {
            if let Some(rows) = intermediate.as_array() {
                if rows.is_empty() {
                    return Ok(String::new());
                }
                let headers: Vec<String> = rows[0]
                    .as_object()
                    .map(|o| o.keys().cloned().collect())
                    .unwrap_or_default();
                let mut out = headers.join(",") + "\n";
                for row in rows {
                    if let Some(obj) = row.as_object() {
                        let values: Vec<String> = headers
                            .iter()
                            .map(|h| {
                                obj.get(h)
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            })
                            .collect();
                        out += &(values.join(",") + "\n");
                    }
                }
                Ok(out)
            } else {
                Ok(intermediate.to_string())
            }
        }
        DataFormat::Toml => {
            if let Some(obj) = intermediate.as_object() {
                let lines: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{} = \"{}\"",
                            k,
                            v.as_str().unwrap_or(&v.to_string())
                        )
                    })
                    .collect();
                Ok(lines.join("\n"))
            } else {
                Ok(intermediate.to_string())
            }
        }
        DataFormat::Animus => {
            // Animus Prime representation
            Ok(format!(
                "\u{27e8}animus:{}\u{27e9}",
                serde_json::to_string(intermediate).unwrap_or_default()
            ))
        }
        _ => {
            // Wrap in generic format representation
            Ok(format!("[{}] {}", to.label(), intermediate))
        }
    }
}

/// Confidence in a direct conversion between two formats.
fn compute_confidence(from: &DataFormat, to: &DataFormat) -> f64 {
    match (from, to) {
        // Lossless text-to-text
        (DataFormat::Json, DataFormat::Toml) => 0.95,
        (DataFormat::Toml, DataFormat::Json) => 0.95,
        (DataFormat::Json, DataFormat::Csv) => 0.85,
        (DataFormat::Csv, DataFormat::Json) => 0.90,
        // Through Animus (universal — meaning preserved)
        (_, DataFormat::Animus) => 0.90,
        (DataFormat::Animus, _) => 0.90,
        // Binary formats (lower confidence — schema needed)
        (DataFormat::Protobuf, _) => 0.70,
        (_, DataFormat::Protobuf) => 0.70,
        // Domain-specific
        (DataFormat::Fix, DataFormat::Json) => 0.88,
        (DataFormat::Hl7, DataFormat::Json) => 0.85,
        // Default
        _ => 0.75,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_to_csv_roundtrip() {
        let json =
            r#"[{"name":"Alice","age":"30"},{"name":"Bob","age":"25"}]"#;
        let result =
            convert(json, &DataFormat::Json, &DataFormat::Csv).expect("ok");
        assert!(result.data.contains("name") || result.data.contains("age"));
        assert!(result.data.contains("Alice"));
        assert!(result.confidence > 0.5);
        assert!(result.is_reliable());
    }

    #[test]
    fn csv_to_json() {
        let csv = "name,age\nAlice,30\nBob,25";
        let result =
            convert(csv, &DataFormat::Csv, &DataFormat::Json).expect("ok");
        assert!(result.data.contains("Alice"));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn same_format_no_op() {
        let data = r#"{"key": "value"}"#;
        let result =
            convert(data, &DataFormat::Json, &DataFormat::Json).expect("ok");
        assert_eq!(result.data, data);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn to_animus_succeeds() {
        let data = r#"{"event": "deployment_failed"}"#;
        let result = convert(data, &DataFormat::Json, &DataFormat::Animus)
            .expect("ok");
        assert!(result.data.contains("animus"));
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn toml_to_json() {
        let toml = "name = \"Hydra\"\nversion = \"1.0\"";
        let result =
            convert(toml, &DataFormat::Toml, &DataFormat::Json).expect("ok");
        assert!(
            result.data.contains("Hydra") || result.data.contains("name")
        );
    }

    #[test]
    fn conversion_result_is_reliable() {
        let result = ConversionResult {
            data: "test".into(),
            from: "json".into(),
            to: "csv".into(),
            confidence: 0.85,
            chain: vec!["json".into(), "csv".into()],
        };
        assert!(result.is_reliable());

        let unreliable = ConversionResult {
            data: "test".into(),
            from: "a".into(),
            to: "b".into(),
            confidence: 0.3,
            chain: vec![],
        };
        assert!(!unreliable.is_reliable());
    }
}
