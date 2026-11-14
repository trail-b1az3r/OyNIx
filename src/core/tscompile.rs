//! TypeScript → JavaScript for OyNIx extensions (`code.ts`), using oxc.
//! Types are erased and TypeScript-only syntax (enums, namespaces,
//! parameter properties…) is lowered; the output targets the Chromium in
//! Qt WebEngine, so no other down-levelling is needed. There's no type
//! checking: like `tsc --noCheck`, code with type errors still runs.

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};
use std::path::Path;

/// Compile TypeScript source. Errors are rendered with the offending line.
pub fn transpile(source: &str, file_name: &str) -> Result<String, String> {
    let src = source.to_string();
    let name = file_name.to_string();
    // A compiler bug must not take the browser down with it.
    std::panic::catch_unwind(move || transpile_inner(&src, &name)).unwrap_or_else(|_| Err(format!("{file_name}: the TypeScript compiler failed on this file")))
}

fn render(e: &oxc::diagnostics::OxcDiagnostic, src: &str, file_name: &str) -> String {
    let full = e.clone().with_source_code(src.to_string());
    let text = format!("{full:?}");
    // Prefer a short "file:line:col: message" over the full report.
    let pos = e.labels.first().map(|l| l.offset()).map(|off| {
        let before = src.get(..(off as usize).min(src.len())).unwrap_or(src);
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().map(|s| s.chars().count()).unwrap_or(0) + 1;
        format!("{file_name}:{line}:{col}: ")
    });
    match pos {
        Some(p) => format!("{p}{}", e.message),
        None => text.lines().next().unwrap_or("syntax error").to_string(),
    }
}

fn transpile_inner(src: &str, file_name: &str) -> Result<String, String> {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, src, SourceType::ts()).parse();
    if let Some(e) = parsed.diagnostics.first() {
        return Err(render(e, src, file_name));
    }
    let mut program = parsed.program;
    let scoping = SemanticBuilder::new().with_enum_eval(true).build(&program).semantic.into_scoping();
    let out = Transformer::new(&alloc, Path::new(file_name), &TransformOptions::default()).build_with_scoping(scoping, &mut program);
    if let Some(e) = out.diagnostics.first() {
        return Err(render(e, src, file_name));
    }
    Ok(Codegen::new().build(&program).code)
}

#[cfg(test)]
mod tests {
    use super::transpile;

    #[test]
    fn erases_types() {
        let js = transpile(
            "interface P { x: number }\nenum C { A = 1, B }\nconst f = (p: P): number => (p.x as number) + C.B;\nclass K { private z: number = 2; m<T>(t: T): T { return t } }\nexport {};",
            "code.ts",
        )
        .unwrap();
        assert!(!js.contains("interface") && !js.contains(": number") && !js.contains("private"));
        assert!(js.contains("C[C[\"B\"] = 2]"));
    }

    #[test]
    fn plain_javascript_is_unchanged_in_meaning() {
        let js = transpile("var a = [1, 2].map(function (x) { return x * 2; });", "code.ts").unwrap();
        assert!(js.contains("x * 2"));
    }

    #[test]
    fn reports_position() {
        let e = transpile("const a = 1;\nconst x: = 3;", "code.ts").unwrap_err();
        assert!(e.starts_with("code.ts:2:"), "{e}");
    }
}
