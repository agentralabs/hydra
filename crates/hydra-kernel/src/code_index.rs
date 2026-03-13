//! Regex-based Rust symbol extraction for codebase semantic indexing.
//!
//! Extracts functions, structs, enums, traits, impls, constants, type aliases,
//! modules, and macros from Rust source files without a tree-sitter dependency.

use regex::Regex;
use std::sync::LazyLock;

// ═══════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolType {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    TypeAlias,
    Module,
    Macro,
}

impl SymbolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Const => "const",
            Self::TypeAlias => "type",
            Self::Module => "mod",
            Self::Macro => "macro",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    PubCrate,
    Private,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::PubCrate => "pub_crate",
            Self::Private => "private",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedSymbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub line_number: usize,
    pub visibility: Visibility,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedEdge {
    pub from_symbol: String,
    pub to_symbol: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    Calls,
    Implements,
    Uses,
    Imports,
    Contains,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Implements => "implements",
            Self::Uses => "uses",
            Self::Imports => "imports",
            Self::Contains => "contains",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// REGEX PATTERNS (compiled once)
// ═══════════════════════════════════════════════════════════

static RE_PUB_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+(\w+)").unwrap()
});
static RE_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?struct\s+(\w+)").unwrap()
});
static RE_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?enum\s+(\w+)").unwrap()
});
static RE_TRAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?trait\s+(\w+)").unwrap()
});
static RE_IMPL_FOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(\w+)\s+for\s+(\w+)").unwrap()
});
static RE_IMPL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(\w+)").unwrap()
});
static RE_CONST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?const\s+(\w+)").unwrap()
});
static RE_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?type\s+(\w+)").unwrap()
});
static RE_MOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(pub(?:\(crate\))?\s+)?mod\s+(\w+)").unwrap()
});
static RE_MACRO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*macro_rules!\s+(\w+)").unwrap()
});
static RE_USE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*use\s+(?:crate|super)::(\w+)").unwrap()
});
static RE_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\w+)\s*\(").unwrap()
});
static RE_DOC_COMMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*///(.*)$").unwrap()
});

// ═══════════════════════════════════════════════════════════
// EXTRACTOR
// ═══════════════════════════════════════════════════════════

pub struct CodeIndexer;

impl CodeIndexer {
    /// Extract all symbols from Rust source code.
    pub fn extract_symbols(source: &str) -> Vec<ExtractedSymbol> {
        let lines: Vec<&str> = source.lines().collect();
        let mut symbols = Vec::new();
        let mut doc_lines: Vec<String> = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;

            // Accumulate doc comments
            if let Some(cap) = RE_DOC_COMMENT.captures(line) {
                doc_lines.push(cap[1].trim().to_string());
                continue;
            }

            let doc = if doc_lines.is_empty() {
                None
            } else {
                Some(doc_lines.join("\n"))
            };

            if let Some(sym) = Self::try_match_symbol(line, line_num, &doc) {
                symbols.push(sym);
            }

            // Reset doc accumulator on non-doc, non-empty lines
            if !line.trim().is_empty() {
                doc_lines.clear();
            }
        }

        symbols
    }

    fn try_match_symbol(
        line: &str,
        line_num: usize,
        doc: &Option<String>,
    ) -> Option<ExtractedSymbol> {
        // Order matters: check impl-for before bare impl
        if let Some(cap) = RE_IMPL_FOR.captures(line) {
            return Some(ExtractedSymbol {
                name: cap[2].to_string(),
                symbol_type: SymbolType::Impl,
                line_number: line_num,
                visibility: Visibility::Private,
                signature: Some(line.trim().to_string()),
                doc_comment: doc.clone(),
            });
        }

        let patterns: &[(
            &LazyLock<Regex>,
            SymbolType,
        )] = &[
            (&RE_PUB_FN, SymbolType::Function),
            (&RE_STRUCT, SymbolType::Struct),
            (&RE_ENUM, SymbolType::Enum),
            (&RE_TRAIT, SymbolType::Trait),
            (&RE_CONST, SymbolType::Const),
            (&RE_TYPE, SymbolType::TypeAlias),
            (&RE_MOD, SymbolType::Module),
        ];

        for (re, sym_type) in patterns {
            if let Some(cap) = re.captures(line) {
                let vis_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let vis = parse_visibility(vis_str);
                let name = cap[3].to_string();
                return Some(ExtractedSymbol {
                    name,
                    symbol_type: sym_type.clone(),
                    line_number: line_num,
                    visibility: vis,
                    signature: Some(line.trim().to_string()),
                    doc_comment: doc.clone(),
                });
            }
        }

        // Bare impl (after impl-for check)
        if let Some(cap) = RE_IMPL.captures(line) {
            let name = cap[1].to_string();
            // Skip if it matched "impl Trait for ..."
            if !RE_IMPL_FOR.is_match(line) {
                return Some(ExtractedSymbol {
                    name,
                    symbol_type: SymbolType::Impl,
                    line_number: line_num,
                    visibility: Visibility::Private,
                    signature: Some(line.trim().to_string()),
                    doc_comment: doc.clone(),
                });
            }
        }

        // Macro
        if let Some(cap) = RE_MACRO.captures(line) {
            return Some(ExtractedSymbol {
                name: cap[1].to_string(),
                symbol_type: SymbolType::Macro,
                line_number: line_num,
                visibility: Visibility::Private,
                signature: Some(line.trim().to_string()),
                doc_comment: doc.clone(),
            });
        }

        None
    }

    /// Extract edges (imports and basic call detection) from source.
    pub fn extract_edges(
        source: &str,
        file_symbols: &[ExtractedSymbol],
    ) -> Vec<ExtractedEdge> {
        let mut edges = Vec::new();
        let symbol_names: Vec<&str> =
            file_symbols.iter().map(|s| s.name.as_str()).collect();

        // Build a set of known symbol names for call detection
        let known: std::collections::HashSet<&str> =
            symbol_names.iter().copied().collect();

        let mut current_fn: Option<&str> = None;

        for line in source.lines() {
            // Track current function scope
            if let Some(cap) = RE_PUB_FN.captures(line) {
                current_fn = Some(
                    // We need owned data but we match against file_symbols
                    file_symbols
                        .iter()
                        .find(|s| s.name == cap[3])
                        .map(|s| s.name.as_str())
                        .unwrap_or(""),
                );
            }

            // Import edges
            if let Some(cap) = RE_USE.captures(line) {
                let imported = &cap[1];
                edges.push(ExtractedEdge {
                    from_symbol: String::new(), // file-level import
                    to_symbol: imported.to_string(),
                    edge_type: EdgeType::Imports,
                });
            }

            // Call edges (within function bodies)
            if let Some(fn_name) = current_fn {
                if !fn_name.is_empty() {
                    for cap in RE_CALL.captures_iter(line) {
                        let callee = &cap[1];
                        if known.contains(callee) && callee != fn_name {
                            edges.push(ExtractedEdge {
                                from_symbol: fn_name.to_string(),
                                to_symbol: callee.to_string(),
                                edge_type: EdgeType::Calls,
                            });
                        }
                    }
                }
            }
        }

        edges
    }
}

// ═══════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════

fn parse_visibility(s: &str) -> Visibility {
    let trimmed = s.trim();
    if trimmed.starts_with("pub(crate)") {
        Visibility::PubCrate
    } else if trimmed.starts_with("pub") {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

#[cfg(test)]
#[path = "code_index_tests.rs"]
mod tests;
