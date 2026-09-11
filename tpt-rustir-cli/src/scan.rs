//! Source scanning: find `#[tpt::spec("…")]` / `#[tpt::verify("…")]`
//! attributes in Rust source files.
//!
//! The CLI never expands the proc-macros; it locates the attributes on the
//! (unexpanded) source with `syn`, extracts the spec strings, and hands them
//! to the same `tpt_rustir_solver::specs` pipeline the macros use — so the
//! CLI and the build pipeline verify with one implementation.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::Visit;

/// A discovered tpt attribute.
#[derive(Debug, Clone)]
pub struct SpecAttr {
    /// `spec` = attached to a function; `verify` = standalone check.
    pub kind: &'static str,
    /// 1-based line of the attribute in the file.
    pub line: usize,
    /// The spec string (empty for a bare `#[tpt::verify]` marker).
    pub spec: String,
    /// The attributed function's name, if any.
    pub fn_name: Option<String>,
}

/// Scan one Rust source file. Returns an error when the file itself doesn't
/// parse as Rust.
pub fn scan_source(path: &Path, src: &str) -> Result<Vec<SpecAttr>, String> {
    let file = syn::parse_file(src).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut visitor = SpecVisitor::default();
    visitor.visit_file(&file);
    Ok(visitor.attrs)
}

#[derive(Default)]
struct SpecVisitor {
    attrs: Vec<SpecAttr>,
    fn_stack: Vec<Option<String>>,
}

impl<'ast> Visit<'ast> for SpecVisitor {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.fn_stack.push(Some(item.sig.ident.to_string()));
        for attr in &item.attrs {
            if let Some(found) = classify_attr(attr) {
                self.attrs.push(SpecAttr {
                    fn_name: Some(item.sig.ident.to_string()),
                    line: attr.span().start().line,
                    ..found
                });
            }
        }
        syn::visit::visit_item_fn(self, item);
        self.fn_stack.pop();
    }

    fn visit_expr_macro(&mut self, _mac: &'ast syn::ExprMacro) {}
}

/// Classify one attribute into a `SpecAttr` skeleton, if it is a tpt one.
fn classify_attr(attr: &syn::Attribute) -> Option<SpecAttr> {
    let segs: Vec<String> = attr
        .path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let seg_strs: Vec<&str> = segs.iter().map(String::as_str).collect();
    let (kind, last) = match seg_strs.as_slice() {
        [one @ ("spec" | "verify")] => (
            if *one == "spec" { "spec" } else { "verify" },
            one.to_string(),
        ),
        ["tpt", tail @ ("spec" | "verify")] => (
            if *tail == "spec" { "spec" } else { "verify" },
            tail.to_string(),
        ),
        _ => return None,
    };
    let _ = last;

    // Extract the spec string, if the attribute carries one.
    let spec = attr
        .parse_args::<syn::LitStr>()
        .ok()
        .map(|lit| lit.value())
        .unwrap_or_default();

    Some(SpecAttr {
        kind,
        line: 0, // filled in by the caller from the attribute span
        spec,
        fn_name: None,
    })
}

/// Recursively collect `*.rs` files under `root`, skipping `target/`, `.git/`
/// and hidden directories.
pub fn collect_rs_files(root: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                stack.push(path);
            } else if name.ends_with(".rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_spec_and_verify_attrs() {
        let src = r##"
            #[tpt::spec("\\x : Nat => succ x")]
            #[tpt::verify]
            fn f(x: u32) -> u32 { x + 1 }

            #[tpt::spec("(+ a b) = (+ b a)")]
            fn g() {}

            #[spec("bare")]
            fn h() {}
        "##;
        let attrs = scan_source(Path::new("t.rs"), src).unwrap();
        assert_eq!(attrs.len(), 4);
        assert_eq!(attrs[0].kind, "spec");
        assert_eq!(attrs[0].spec, "\\x : Nat => succ x");
        assert_eq!(attrs[0].fn_name.as_deref(), Some("f"));
        assert_eq!(attrs[1].kind, "verify");
        assert_eq!(attrs[1].spec, "");
        assert_eq!(attrs[1].fn_name.as_deref(), Some("f"));
        assert_eq!(attrs[2].kind, "spec");
        assert_eq!(attrs[2].spec, "(+ a b) = (+ b a)");
        assert_eq!(attrs[2].fn_name.as_deref(), Some("g"));
        assert_eq!(attrs[3].kind, "spec"); // bare `spec` path also matches
        assert_eq!(attrs[3].spec, "bare");
        assert_eq!(attrs[3].fn_name.as_deref(), Some("h"));
    }
}