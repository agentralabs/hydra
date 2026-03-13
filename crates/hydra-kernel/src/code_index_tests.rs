use super::*;

#[test]
fn test_extract_pub_function() {
    let src = "pub fn hello_world() -> String {\n    String::new()\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "hello_world");
    assert_eq!(symbols[0].symbol_type, SymbolType::Function);
    assert_eq!(symbols[0].visibility, Visibility::Public);
    assert_eq!(symbols[0].line_number, 1);
}

#[test]
fn test_extract_pub_crate_function() {
    let src = "pub(crate) fn internal_helper() {}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "internal_helper");
    assert_eq!(symbols[0].visibility, Visibility::PubCrate);
}

#[test]
fn test_extract_private_function() {
    let src = "fn private_fn() {}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "private_fn");
    assert_eq!(symbols[0].visibility, Visibility::Private);
}

#[test]
fn test_extract_async_function() {
    let src = "pub async fn fetch_data() -> Result<()> {}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "fetch_data");
    assert_eq!(symbols[0].symbol_type, SymbolType::Function);
}

#[test]
fn test_extract_struct() {
    let src = "pub struct MyConfig {\n    pub name: String,\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "MyConfig");
    assert_eq!(symbols[0].symbol_type, SymbolType::Struct);
    assert_eq!(symbols[0].visibility, Visibility::Public);
}

#[test]
fn test_extract_enum() {
    let src = "pub(crate) enum Status {\n    Active,\n    Inactive,\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "Status");
    assert_eq!(symbols[0].symbol_type, SymbolType::Enum);
    assert_eq!(symbols[0].visibility, Visibility::PubCrate);
}

#[test]
fn test_extract_trait() {
    let src = "pub trait Handler {\n    fn handle(&self);\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    // trait + fn inside
    let trait_sym = symbols.iter().find(|s| s.symbol_type == SymbolType::Trait);
    assert!(trait_sym.is_some());
    assert_eq!(trait_sym.unwrap().name, "Handler");
}

#[test]
fn test_extract_impl_block() {
    let src = "impl MyConfig {\n    pub fn new() -> Self { Self {} }\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    let impl_sym = symbols.iter().find(|s| s.symbol_type == SymbolType::Impl);
    assert!(impl_sym.is_some());
    assert_eq!(impl_sym.unwrap().name, "MyConfig");
}

#[test]
fn test_extract_impl_for_trait() {
    let src = "impl Handler for MyConfig {\n    fn handle(&self) {}\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    let impl_sym = symbols.iter().find(|s| s.symbol_type == SymbolType::Impl);
    assert!(impl_sym.is_some());
    assert_eq!(impl_sym.unwrap().name, "MyConfig");
}

#[test]
fn test_extract_const() {
    let src = "pub const MAX_SIZE: usize = 1024;\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "MAX_SIZE");
    assert_eq!(symbols[0].symbol_type, SymbolType::Const);
}

#[test]
fn test_extract_type_alias() {
    let src = "pub type Result<T> = std::result::Result<T, Error>;\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "Result");
    assert_eq!(symbols[0].symbol_type, SymbolType::TypeAlias);
}

#[test]
fn test_extract_module() {
    let src = "pub mod helpers;\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "helpers");
    assert_eq!(symbols[0].symbol_type, SymbolType::Module);
}

#[test]
fn test_extract_macro() {
    let src = "macro_rules! my_macro {\n    () => {};\n}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "my_macro");
    assert_eq!(symbols[0].symbol_type, SymbolType::Macro);
}

#[test]
fn test_extract_doc_comments() {
    let src = "/// Initialize the system.\n/// Sets up all components.\npub fn init() {}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "init");
    let doc = symbols[0].doc_comment.as_ref().unwrap();
    assert!(doc.contains("Initialize the system."));
    assert!(doc.contains("Sets up all components."));
}

#[test]
fn test_empty_source() {
    let symbols = CodeIndexer::extract_symbols("");
    assert!(symbols.is_empty());
}

#[test]
fn test_edge_extraction_imports() {
    let src = "use crate::config;\nuse super::helpers;\npub fn run() {}\n";
    let symbols = CodeIndexer::extract_symbols(src);
    let edges = CodeIndexer::extract_edges(src, &symbols);
    let imports: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::Imports).collect();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].to_symbol, "config");
    assert_eq!(imports[1].to_symbol, "helpers");
}

#[test]
fn test_edge_extraction_calls() {
    let src = "\
pub fn helper() -> i32 { 42 }

pub fn main_fn() {
    let x = helper();
}
";
    let symbols = CodeIndexer::extract_symbols(src);
    let edges = CodeIndexer::extract_edges(src, &symbols);
    let calls: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::Calls).collect();
    assert!(!calls.is_empty());
    assert!(calls.iter().any(|e| e.from_symbol == "main_fn" && e.to_symbol == "helper"));
}

#[test]
fn test_multiple_symbols_in_file() {
    let src = "\
pub struct Config {}
pub enum Mode { Fast, Slow }
pub fn run() {}
const LIMIT: usize = 10;
pub trait Engine {}
impl Config {}
";
    let symbols = CodeIndexer::extract_symbols(src);
    assert!(symbols.len() >= 5);
    let types: Vec<_> = symbols.iter().map(|s| &s.symbol_type).collect();
    assert!(types.contains(&&SymbolType::Struct));
    assert!(types.contains(&&SymbolType::Enum));
    assert!(types.contains(&&SymbolType::Function));
    assert!(types.contains(&&SymbolType::Trait));
    assert!(types.contains(&&SymbolType::Impl));
}

#[test]
fn test_symbol_type_as_str() {
    assert_eq!(SymbolType::Function.as_str(), "function");
    assert_eq!(SymbolType::Struct.as_str(), "struct");
    assert_eq!(SymbolType::Macro.as_str(), "macro");
}

#[test]
fn test_visibility_as_str() {
    assert_eq!(Visibility::Public.as_str(), "public");
    assert_eq!(Visibility::PubCrate.as_str(), "pub_crate");
    assert_eq!(Visibility::Private.as_str(), "private");
}

#[test]
fn test_edge_type_as_str() {
    assert_eq!(EdgeType::Calls.as_str(), "calls");
    assert_eq!(EdgeType::Imports.as_str(), "imports");
    assert_eq!(EdgeType::Implements.as_str(), "implements");
}
