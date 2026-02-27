//! Codebase indexer — symbol extraction from source files.
//!
//! Extracts code symbols (functions, structs, types, impls, modules) from
//! source files using pattern matching. This provides the static context layer
//! for the context engine.
//!
//! ## Architecture
//!
//! The indexer walks a directory tree, identifies source files by extension,
//! and extracts symbols using language-specific patterns. The extracted symbols
//! are stored in an in-memory index that can be queried by name, kind, or path.
//!
//! In a future version, this will use tree-sitter AST parsing for more accurate
//! symbol extraction. The current implementation uses regex patterns as a
//! lightweight, dependency-free starting point.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A code symbol extracted from a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name (e.g. "Daemon", "run", "AppConfig").
    pub name: String,
    /// Kind of symbol.
    pub kind: SymbolKind,
    /// File path where the symbol is defined.
    pub path: PathBuf,
    /// Line number (1-indexed).
    pub line: u32,
    /// The full signature/declaration line.
    pub signature: String,
    /// Optional documentation comment.
    pub doc: Option<String>,
}

/// Kind of code symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Const,
    Module,
    Macro,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Impl => write!(f, "impl"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Const => write!(f, "const"),
            SymbolKind::Module => write!(f, "module"),
            SymbolKind::Macro => write!(f, "macro"),
        }
    }
}

/// In-memory symbol index.
pub struct SymbolIndex {
    /// All symbols, keyed by file path.
    by_path: HashMap<PathBuf, Vec<Symbol>>,
    /// Flat list of all symbols.
    all: Vec<Symbol>,
}

impl SymbolIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self {
            by_path: HashMap::new(),
            all: Vec::new(),
        }
    }

    /// Index a single file by extracting symbols from its content.
    pub fn index_file(&mut self, path: &Path, content: &str) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let symbols = match ext {
            "rs" => extract_rust_symbols(path, content),
            "ts" | "tsx" | "js" | "jsx" => extract_typescript_symbols(path, content),
            "py" => extract_python_symbols(path, content),
            "go" => extract_go_symbols(path, content),
            _ => Vec::new(),
        };

        for sym in &symbols {
            self.all.push(sym.clone());
        }
        if !symbols.is_empty() {
            self.by_path.insert(path.to_path_buf(), symbols);
        }
    }

    /// Index all supported files under a directory tree.
    pub fn index_directory(&mut self, root: &Path) -> std::io::Result<usize> {
        let mut count = 0;
        index_dir_recursive(root, &mut |path, content| {
            self.index_file(path, content);
            count += 1;
        })?;
        Ok(count)
    }

    /// Search symbols by name (case-insensitive substring match).
    pub fn search(&self, query: &str) -> Vec<&Symbol> {
        let query_lower = query.to_lowercase();
        self.all
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get symbols by kind.
    pub fn by_kind(&self, kind: SymbolKind) -> Vec<&Symbol> {
        self.all.iter().filter(|s| s.kind == kind).collect()
    }

    /// Get symbols in a specific file.
    pub fn in_file(&self, path: &Path) -> Vec<&Symbol> {
        self.by_path
            .get(path)
            .map(|syms| syms.iter().collect())
            .unwrap_or_default()
    }

    /// Total number of indexed symbols.
    pub fn len(&self) -> usize {
        self.all.len()
    }

    /// Is the index empty?
    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    /// Number of indexed files.
    pub fn file_count(&self) -> usize {
        self.by_path.len()
    }

    /// Get all symbols.
    pub fn symbols(&self) -> &[Symbol] {
        &self.all
    }

    /// Generate a summary string for use as context.
    pub fn summary(&self) -> String {
        let mut output = String::new();

        let mut by_kind: HashMap<SymbolKind, usize> = HashMap::new();
        for sym in &self.all {
            *by_kind.entry(sym.kind).or_default() += 1;
        }

        output.push_str(&format!(
            "Codebase index: {} symbols in {} files\n",
            self.len(),
            self.file_count()
        ));
        for (kind, count) in &by_kind {
            output.push_str(&format!("  {kind}: {count}\n"));
        }
        output
    }
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Walk a directory recursively, calling `visitor` for each supported source file.
fn index_dir_recursive(dir: &Path, visitor: &mut dyn FnMut(&Path, &str)) -> std::io::Result<()> {
    let supported_extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go"];

    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden directories and common non-source directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.starts_with('.') || name == "target" || name == "node_modules")
        {
            continue;
        }

        if path.is_dir() {
            index_dir_recursive(&path, visitor)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && supported_extensions.contains(&ext)
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            visitor(&path, &content);
        }
    }
    Ok(())
}

// ── Language-specific symbol extraction ─────────────────────────────────

/// Extract symbols from Rust source code.
fn extract_rust_symbols(path: &Path, content: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut doc_comment = String::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Collect doc comments
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            doc_comment.push_str(trimmed.trim_start_matches('/').trim());
            doc_comment.push('\n');
            continue;
        }

        let doc = if doc_comment.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut doc_comment).trim().to_string())
        };

        // pub fn / fn
        if let Some(name) = extract_after_keyword(trimmed, "fn ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "struct ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "enum ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Enum,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "trait ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Trait,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc,
            });
        } else if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
            let name = trimmed
                .trim_start_matches("impl")
                .trim()
                .split(|c: char| c == '{' || c == '<' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Impl,
                    path: path.to_path_buf(),
                    line: (line_num + 1) as u32,
                    signature: trimmed.to_string(),
                    doc,
                });
            }
        } else if let Some(name) = extract_after_keyword(trimmed, "type ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Type,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "mod ") {
            if !trimmed.contains("//")
                || trimmed.find("mod").unwrap() < trimmed.find("//").unwrap_or(usize::MAX)
            {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Module,
                    path: path.to_path_buf(),
                    line: (line_num + 1) as u32,
                    signature: trimmed.to_string(),
                    doc,
                });
            }
        } else {
            // Don't carry doc comments across non-matching lines
            doc_comment.clear();
        }
    }
    symbols
}

/// Extract symbols from TypeScript/JavaScript source code.
fn extract_typescript_symbols(path: &Path, content: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if let Some(name) = extract_after_keyword(trimmed, "function ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "class ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "interface ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Trait,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "type ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Type,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        }
    }
    symbols
}

/// Extract symbols from Python source code.
fn extract_python_symbols(path: &Path, content: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if let Some(name) = extract_after_keyword(trimmed, "def ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "class ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        }
    }
    symbols
}

/// Extract symbols from Go source code.
fn extract_go_symbols(path: &Path, content: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if let Some(name) = extract_after_keyword(trimmed, "func ") {
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        } else if let Some(name) = extract_after_keyword(trimmed, "type ") {
            let kind = if trimmed.contains("struct") {
                SymbolKind::Struct
            } else if trimmed.contains("interface") {
                SymbolKind::Trait
            } else {
                SymbolKind::Type
            };
            symbols.push(Symbol {
                name,
                kind,
                path: path.to_path_buf(),
                line: (line_num + 1) as u32,
                signature: trimmed.to_string(),
                doc: None,
            });
        }
    }
    symbols
}

/// Extract the identifier after a keyword in a line.
///
/// e.g. `extract_after_keyword("pub fn run(", "fn ")` → `Some("run")`
fn extract_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let idx = line.find(keyword)?;
    let after = &line[idx + keyword.len()..];
    let name: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_function() {
        let code = "pub fn run(&self) -> Result<(), Error> {\n";
        let symbols = extract_rust_symbols(Path::new("lib.rs"), code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "run");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_rust_struct() {
        let code = "pub struct Daemon {\n    config: AppConfig,\n}\n";
        let symbols = extract_rust_symbols(Path::new("daemon.rs"), code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Daemon");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn test_extract_rust_enum() {
        let code = "pub enum DaemonError {\n    Startup(String),\n}\n";
        let symbols = extract_rust_symbols(Path::new("daemon.rs"), code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "DaemonError");
        assert_eq!(symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn test_extract_rust_trait() {
        let code = "pub trait LlmProvider: Send + Sync {\n";
        let symbols = extract_rust_symbols(Path::new("provider.rs"), code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "LlmProvider");
        assert_eq!(symbols[0].kind, SymbolKind::Trait);
    }

    #[test]
    fn test_extract_rust_impl() {
        let code = "impl Daemon {\n    pub fn new() -> Self {\n";
        let symbols = extract_rust_symbols(Path::new("daemon.rs"), code);
        assert!(
            symbols
                .iter()
                .any(|s| s.kind == SymbolKind::Impl && s.name == "Daemon")
        );
    }

    #[test]
    fn test_extract_rust_module() {
        let code = "pub mod ipc;\n/// The LLM module.\npub mod llm;\n";
        let symbols = extract_rust_symbols(Path::new("lib.rs"), code);
        assert_eq!(symbols.len(), 2);
        assert!(symbols.iter().any(|s| s.name == "ipc"));
        assert!(symbols.iter().any(|s| s.name == "llm"));
    }

    #[test]
    fn test_extract_rust_doc_comments() {
        let code = "/// This is the daemon.\n/// It runs things.\npub struct Daemon {}\n";
        let symbols = extract_rust_symbols(Path::new("daemon.rs"), code);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].doc.as_ref().unwrap().contains("daemon"));
    }

    #[test]
    fn test_extract_typescript_function() {
        let code = "export function handleRequest(req: Request): Response {\n";
        let symbols = extract_typescript_symbols(Path::new("handler.ts"), code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "handleRequest");
    }

    #[test]
    fn test_extract_python_class() {
        let code = "class MyModel:\n    def __init__(self):\n        pass\n";
        let symbols = extract_python_symbols(Path::new("model.py"), code);
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MyModel" && s.kind == SymbolKind::Struct)
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "__init__" && s.kind == SymbolKind::Function)
        );
    }

    #[test]
    fn test_symbol_index_search() {
        let mut index = SymbolIndex::new();
        let code = "pub fn daemon_run() {}\npub fn daemon_stop() {}\npub fn config_load() {}\n";
        index.index_file(Path::new("daemon.rs"), code);

        let results = index.search("daemon");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_symbol_index_by_kind() {
        let mut index = SymbolIndex::new();
        let code = "pub struct Daemon {}\npub fn run() {}\npub enum Error {}\n";
        index.index_file(Path::new("lib.rs"), code);

        assert_eq!(index.by_kind(SymbolKind::Struct).len(), 1);
        assert_eq!(index.by_kind(SymbolKind::Function).len(), 1);
        assert_eq!(index.by_kind(SymbolKind::Enum).len(), 1);
    }

    #[test]
    fn test_symbol_index_summary() {
        let mut index = SymbolIndex::new();
        let code = "pub struct Daemon {}\npub fn run() {}\n";
        index.index_file(Path::new("lib.rs"), code);

        let summary = index.summary();
        assert!(summary.contains("2 symbols"));
        assert!(summary.contains("1 files"));
    }

    #[test]
    fn test_extract_after_keyword() {
        assert_eq!(
            extract_after_keyword("pub fn run()", "fn "),
            Some("run".to_string())
        );
        assert_eq!(
            extract_after_keyword("struct Foo {", "struct "),
            Some("Foo".to_string())
        );
        assert_eq!(extract_after_keyword("let x = 5;", "fn "), None);
    }
}
