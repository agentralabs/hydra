//! O20 Document Vision — read screenshots, PDFs, diagrams, charts, spreadsheets.
//! 3-tier cascade: OCR (local, 0 tokens) → structural analysis → vision LLM fallback.
//! Reuses crate::ocr for Tier 1. PDF via pdftotext subprocess.

use crate::errors::DesktopError;
use std::path::Path;

// ── Types ──

/// Detected document type.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType { Screenshot, Pdf, Image, Csv, Table, Unknown }

impl DocumentType {
    pub fn label(&self) -> &'static str {
        match self { Self::Screenshot => "screenshot", Self::Pdf => "pdf",
            Self::Image => "image", Self::Csv => "csv",
            Self::Table => "table", Self::Unknown => "unknown" }
    }
}

/// Extracted document content.
#[derive(Debug, Clone)]
pub struct DocumentContent {
    pub doc_type: DocumentType,
    pub text: String,
    pub structure: Option<TableData>,
    pub confidence: f64,
    pub tier_used: u8,
}

/// Structured table data extracted from document.
#[derive(Debug, Clone)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub summary: String,
}

// ── Detection ──

/// Detect document type from file extension.
pub fn detect_type(path: &str) -> DocumentType {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => DocumentType::Pdf,
        "png" | "jpg" | "jpeg" | "bmp" | "tiff" => DocumentType::Screenshot,
        "gif" | "webp" | "svg" => DocumentType::Image,
        "csv" | "tsv" => DocumentType::Csv,
        _ => DocumentType::Unknown,
    }
}

// ── Main Pipeline ──

/// Process a document through the 3-tier cascade.
/// Tier 1: OCR text extraction. Tier 2: structural analysis. Tier 3: needs vision LLM (signaled).
pub fn process_document(path: &str) -> Result<DocumentContent, DesktopError> {
    if !Path::new(path).exists() {
        return Err(DesktopError::Io(format!("File not found: {path}")));
    }
    // EC-27.4: Reject very large files to prevent OOM
    const MAX_DOC_BYTES: u64 = 50_000_000; // 50MB
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_DOC_BYTES {
            return Err(DesktopError::Io(format!("File too large ({:.1}MB, max 50MB)", meta.len() as f64 / 1_000_000.0)));
        }
    }
    let doc_type = detect_type(path);

    // Tier 1: Extract text
    let text = match &doc_type {
        DocumentType::Pdf => extract_pdf_text(path)?,
        DocumentType::Csv => std::fs::read_to_string(path)
            .map_err(|e| DesktopError::Io(format!("{e}")))?,
        DocumentType::Screenshot | DocumentType::Image => {
            let regions = crate::ocr::ocr_screenshot(path)?;
            regions.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join(" ")
        }
        _ => String::new(),
    };

    // Tier 2: Structural analysis — detect tables
    let structure = detect_table(&text);
    let tier_used = if !text.is_empty() { if structure.is_some() { 2 } else { 1 } } else { 3 };
    let confidence = match tier_used { 1 => 0.7, 2 => 0.85, _ => 0.5 };

    eprintln!("hydra-document: processed {} as {:?} (tier {}, {}chars)",
        path, doc_type, tier_used, text.len());

    Ok(DocumentContent { doc_type, text, structure, confidence, tier_used })
}

/// Extract text from PDF using pdftotext subprocess.
pub fn extract_pdf_text(path: &str) -> Result<String, DesktopError> {
    let output = std::process::Command::new("pdftotext").arg(path).arg("-")
        .output().map_err(|e| DesktopError::Io(format!("pdftotext: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(DesktopError::Io("pdftotext failed".into()))
    }
}

// ── Table Detection ──

/// Heuristic table detection from OCR/extracted text.
pub fn detect_table(text: &str) -> Option<TableData> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 { return None; }
    // Detect delimiter: tab, comma, or pipe
    let delim = if lines[0].contains('\t') { '\t' }
        else if lines[0].contains('|') { '|' }
        else if lines[0].contains(',') && lines[0].matches(',').count() >= 2 { ',' }
        else { return None; };
    let headers: Vec<String> = lines[0].split(delim).map(|s| s.trim().to_string()).collect();
    if headers.len() < 2 { return None; }
    let rows: Vec<Vec<String>> = lines[1..].iter()
        .map(|l| l.split(delim).map(|s| s.trim().to_string()).collect())
        .filter(|r: &Vec<String>| r.len() >= headers.len().saturating_sub(1))
        .collect();
    if rows.is_empty() { return None; }
    let summary = format!("{} columns, {} rows", headers.len(), rows.len());
    Some(TableData { headers, rows, summary })
}

/// Compact summary for enrichment injection.
pub fn summarize_content(content: &DocumentContent) -> String {
    let mut summary = format!("[{} document, tier {}]", content.doc_type.label(), content.tier_used);
    if let Some(ref table) = content.structure {
        summary.push_str(&format!(" Table: {}", table.summary));
    }
    let preview = if content.text.len() > 500 { &content.text[..500] } else { &content.text };
    if !preview.is_empty() { summary.push_str(&format!("\n{preview}")); }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_type_pdf() { assert_eq!(detect_type("contract.pdf"), DocumentType::Pdf); }

    #[test]
    fn detect_type_png() { assert_eq!(detect_type("error.png"), DocumentType::Screenshot); }

    #[test]
    fn detect_type_unknown() { assert_eq!(detect_type("readme.md"), DocumentType::Unknown); }

    #[test]
    fn table_detection_tsv() {
        let text = "Name\tAge\tCity\nAlice\t30\tNYC\nBob\t25\tSF";
        let table = detect_table(text);
        assert!(table.is_some());
        let t = table.unwrap();
        assert_eq!(t.headers.len(), 3);
        assert_eq!(t.rows.len(), 2);
    }

    #[test]
    fn summarize_format() {
        let content = DocumentContent {
            doc_type: DocumentType::Pdf, text: "Hello world".into(),
            structure: None, confidence: 0.7, tier_used: 1,
        };
        let s = summarize_content(&content);
        assert!(s.contains("pdf"));
        assert!(s.contains("Hello world"));
    }

    #[test]
    fn no_table_in_prose() {
        let text = "This is just a regular paragraph with no table structure at all.";
        assert!(detect_table(text).is_none());
    }
}
