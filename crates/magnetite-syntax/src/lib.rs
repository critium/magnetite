//! Turns `.mgt.md` files into an AST.
//!
//! Pipeline (see PLAN.md, "Compiler architecture"):
//! 1. Markdown extraction: recognized tables, lists, headings, and fenced
//!    `mgt` / `rust` blocks, with original line numbers kept.
//! 2. Lexing of code spans and `mgt` blocks (indentation-aware).
//! 3. Recursive-descent parsing into an AST with source spans on every node.

use std::path::{Path, PathBuf};

/// The file extension of a Magnetite module.
pub const EXTENSION: &str = ".mgt.md";

/// One `.mgt.md` file, loaded into memory.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
}

impl SourceFile {
    pub fn load(path: &Path) -> std::io::Result<SourceFile> {
        let text = std::fs::read_to_string(path)?;
        Ok(SourceFile {
            path: path.to_path_buf(),
            text,
        })
    }

    /// The module name: the file name without `.mgt.md` (`orders.mgt.md` → `orders`).
    pub fn module_name(&self) -> &str {
        let file_name = self.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        file_name.strip_suffix(EXTENSION).unwrap_or(file_name)
    }
}

/// Whether `path` names a Magnetite module.
pub fn is_module(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(EXTENSION))
}

/// Finds every `.mgt.md` file under `dir`, sorted by path so output is stable.
pub fn discover(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if is_module(&path) {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name_strips_extension() {
        let file = SourceFile {
            path: PathBuf::from("examples/shop/orders.mgt.md"),
            text: String::new(),
        };
        assert_eq!(file.module_name(), "orders");
    }

    #[test]
    fn only_mgt_md_files_are_modules() {
        assert!(is_module(Path::new("orders.mgt.md")));
        assert!(!is_module(Path::new("README.md")));
        assert!(!is_module(Path::new("orders.mgt")));
    }
}
