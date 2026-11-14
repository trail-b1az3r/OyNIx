//! Check an extension's `graphics.qt` before OyNIx runs it.
//!
//! Panels run in their own QML engine with the network and most modules
//! blocked (see `load_panel` in cpp/native.cpp). That engine can't stop a
//! panel from importing OyNIx's own C++ types, because they're registered
//! process-wide, and the only routes to them are an `import` line or QML
//! compiled at run time. This checker closes both, on the source text:
//!
//! * imports must come from Qt's visual modules;
//! * no run-time QML (`createQmlObject`, `createComponent`, `Loader`);
//! * no way to reach the global object (`this`, `globalThis`, `Function`,
//!   `constructor`, `eval`, `Reflect`…), so `Qt` can only be reached by
//!   name;
//! * no way to climb out of the panel into the browser window that hosts
//!   it (`parent`, `Window.window`, `Overlay`…): panels use ids, and
//!   OyNIx sizes the root item; and
//! * `Qt` itself only for colours, geometry, enums and formatting.
//!
//! The source is tokenised (comments, strings, template literals and
//! regular expressions are understood) so words inside strings and
//! comments don't count.

const ALLOWED_IMPORTS: &[&str] = &[
    "QtQuick", "QtQuick.Layouts", "QtQuick.Controls", "QtQuick.Controls.Basic", "QtQuick.Shapes",
    "QtQuick.Effects", "QtQml", "QtQml.Models",
];

const BANNED: &[&str] = &[
    "this", "globalThis", "eval", "Function", "constructor", "__proto__", "prototype", "Reflect", "Proxy",
    "Loader", "createQmlObject", "createComponent", "import", "require", "XMLHttpRequest", "WorkerScript",
    "WebSocket", "Worker", "include", "getPrototypeOf", "defineProperty", "__defineGetter__", "arguments",
    // The panel sits inside the browser window; these climb out of it into
    // OyNIx's own interface. Panels use ids instead of `parent`.
    "parent", "window", "Window", "ApplicationWindow", "Overlay", "overlay", "contentItem", "activeFocusItem",
    "visualParent", "toolTip", "DropArea", "focusScope",
];

/// `Qt.` members a panel may use (besides enum values, which start with a
/// capital letter).
const QT_ALLOWED: &[&str] = &[
    "binding", "rgba", "hsla", "hsva", "color", "darker", "lighter", "tint", "alpha", "colorEqual", "point", "size",
    "rect", "vector2d", "vector3d", "vector4d", "quaternion", "matrix4x4", "font", "fontFamilies", "formatDate",
    "formatTime", "formatDateTime", "locale", "md5", "btoa", "atob", "callLater", "platform", "styleHints", "isQtObject",
];

#[derive(Debug, PartialEq)]
enum Tok {
    Ident(String),
    Punct(char),
    Other,
}

/// Tokenise QML/JavaScript source, dropping comments and literal contents
/// (but tokenising `${…}` inside template literals).
fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out: Vec<(Tok, usize)> = Vec::new();
    let mut i = 0;
    let mut line = 1;
    // Brace depth at which each open template literal's `${` started.
    let mut templates: Vec<usize> = Vec::new();
    let mut depth = 0usize;
    let regex_may_follow = |out: &Vec<(Tok, usize)>| match out.last() {
        None => true,
        Some((Tok::Punct(p), _)) => "(,=:[!&|?{};+-*%<>~^".contains(*p),
        Some((Tok::Ident(w), _)) => matches!(w.as_str(), "return" | "typeof" | "case" | "do" | "else" | "in" | "of" | "new" | "delete" | "void" | "throw"),
        Some((Tok::Other, _)) => false,
    };
    while i < c.len() {
        let ch = c[i];
        if ch == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&'/')) {
                if c[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            if i >= c.len() {
                return Err(format!("line {line}: unterminated comment"));
            }
            i += 2;
            continue;
        }
        // Closing a `${ … }` returns to the template literal.
        if ch == '}' && templates.last() == Some(&depth) {
            templates.pop();
            i = scan_template(&c, i + 1, &mut line, &mut templates, depth)?;
            out.push((Tok::Other, line));
            continue;
        }
        if ch == '`' {
            i = scan_template(&c, i + 1, &mut line, &mut templates, depth)?;
            out.push((Tok::Other, line));
            continue;
        }
        if ch == '"' || ch == '\'' {
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' {
                    i += 1;
                }
                if i < c.len() && c[i] == '\n' {
                    return Err(format!("line {line}: unterminated string"));
                }
                i += 1;
            }
            if i >= c.len() {
                return Err(format!("line {line}: unterminated string"));
            }
            i += 1;
            out.push((Tok::Other, line));
            continue;
        }
        if ch == '/' && regex_may_follow(&out) {
            i += 1;
            let mut class = false;
            while i < c.len() && (class || c[i] != '/') {
                match c[i] {
                    '\\' => i += 1,
                    '[' => class = true,
                    ']' => class = false,
                    '\n' => return Err(format!("line {line}: unterminated regular expression")),
                    _ => {}
                }
                i += 1;
            }
            if i >= c.len() {
                return Err(format!("line {line}: unterminated regular expression"));
            }
            i += 1;
            while i < c.len() && c[i].is_ascii_alphabetic() {
                i += 1;
            }
            out.push((Tok::Other, line));
            continue;
        }
        if ch.is_alphabetic() || ch == '_' || ch == '$' {
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '$') {
                i += 1;
            }
            out.push((Tok::Ident(c[start..i].iter().collect()), line));
            continue;
        }
        if ch.is_ascii_digit() {
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '.') {
                i += 1;
            }
            out.push((Tok::Other, line));
            continue;
        }
        if ch == '\\' {
            // Unicode escapes in identifiers could spell banned words.
            return Err(format!("line {line}: backslash outside a string"));
        }
        if ch == '{' {
            depth += 1;
        }
        if ch == '}' {
            depth = depth.saturating_sub(1);
        }
        out.push((Tok::Punct(ch), line));
        i += 1;
    }
    if !templates.is_empty() {
        return Err("unterminated template literal".into());
    }
    Ok(out)
}

/// Skip template literal text from `i`; stops after the closing backtick, or
/// just after `${` (recording the brace depth to return to).
fn scan_template(c: &[char], mut i: usize, line: &mut usize, templates: &mut Vec<usize>, depth: usize) -> Result<usize, String> {
    while i < c.len() {
        match c[i] {
            '\\' => i += 2,
            '\n' => {
                *line += 1;
                i += 1;
            }
            '`' => return Ok(i + 1),
            '$' if c.get(i + 1) == Some(&'{') => {
                templates.push(depth);
                return Ok(i + 2);
            }
            _ => i += 1,
        }
    }
    Err("unterminated template literal".into())
}

pub fn check(src: &str) -> Result<(), String> {
    let toks = tokenize(src)?;
    let mut i = 0;
    // Header: `import Module [Version] [as Name]` and `pragma …` lines.
    while i < toks.len() {
        match &toks[i].0 {
            Tok::Ident(w) if w == "pragma" => {
                let l = toks[i].1;
                while i < toks.len() && toks[i].1 == l {
                    i += 1;
                }
            }
            Tok::Ident(w) if w == "import" => {
                let l = toks[i].1;
                let mut module = String::new();
                i += 1;
                if !matches!(toks.get(i), Some((Tok::Ident(_), _))) {
                    return Err(format!("graphics.qt line {l}: only Qt modules can be imported, not files or folders"));
                }
                while let Some((t, tl)) = toks.get(i) {
                    if *tl != l {
                        break;
                    }
                    match t {
                        Tok::Ident(w) if w == "as" => break,
                        Tok::Ident(w) => module.push_str(w),
                        Tok::Punct('.') => module.push('.'),
                        _ => {}
                    }
                    i += 1;
                }
                while i < toks.len() && toks[i].1 == l {
                    i += 1;
                }
                // A trailing version number shows up as ".digits"; drop it.
                let module = module.trim_end_matches('.').to_string();
                if !ALLOWED_IMPORTS.contains(&module.as_str()) {
                    return Err(format!("graphics.qt line {l}: “import {module}” isn’t allowed in panels (allowed: {})", ALLOWED_IMPORTS.join(", ")));
                }
            }
            _ => break,
        }
    }
    for (k, (t, l)) in toks.iter().enumerate().skip(i) {
        let Tok::Ident(w) = t else { continue };
        if BANNED.contains(&w.as_str()) {
            return Err(format!("graphics.qt line {l}: “{w}” isn’t allowed in panels"));
        }
        if w == "Qt" {
            let member = match (toks.get(k + 1), toks.get(k + 2)) {
                (Some((Tok::Punct('.'), _)), Some((Tok::Ident(m), _))) => m.as_str(),
                _ => return Err(format!("graphics.qt line {l}: use Qt only as “Qt.something”")),
            };
            let enum_value = member.chars().next().is_some_and(|c| c.is_uppercase());
            if !enum_value && !QT_ALLOWED.contains(&member) {
                return Err(format!("graphics.qt line {l}: “Qt.{member}” isn’t allowed in panels"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check;

    const OK: &str = r#"import QtQuick
import QtQuick.Controls.Basic 2.15 as C
pragma ComponentBehavior: Bound
// "this" in a comment is fine, and so is Loader here
Rectangle {
    id: root
    color: Qt.rgba(0.2, 0.3, 0.9, 1)
    Item { anchors.fill: root }
    property var re: /a'b"c\/[/]/g
    Text { text: `This is ${extension.id} with ${ {a: 1}.a } items`; horizontalAlignment: Qt.AlignHCenter }
    C.Button { text: "Use this"; onClicked: extension.post(JSON.stringify({ n: 1 / 2 })) }
    Connections { target: extension; function onMessage(p) { var x = JSON.parse(p); Qt.callLater(function () { x.y = 1 }) } }
}"#;

    #[test]
    fn accepts_ordinary_panels() {
        check(OK).unwrap();
    }

    #[test]
    fn rejects_escapes() {
        for (src, why) in [
            ("import OyNIx\nItem {}", "import OyNIx"),
            ("import \"../x\"\nItem {}", "files"),
            ("import QtWebEngine\nItem {}", "QtWebEngine"),
            ("import QtQuick\nItem { Component.onCompleted: Qt.createQmlObject('import OyNIx; Item {}', parent) }", "Qt.createQmlObject"),
            ("import QtQuick\nItem { property var q: Qt }", "Qt only"),
            ("import QtQuick\nItem { property var q: Qt['create' + 'QmlObject'] }", "Qt only"),
            ("import QtQuick\nItem { Loader { source: 'x.qml' } }", "Loader"),
            ("import QtQuick\nItem { function f() { return this } }", "this"),
            ("import QtQuick\nItem { property var f: (function(){}).constructor }", "constructor"),
            ("import QtQuick\nItem { Component.onCompleted: Qt.openUrlExternally('https://x') }", "openUrlExternally"),
            ("import QtQuick\nItem { Component.onCompleted: Qt.quit() }", "Qt.quit"),
            ("import QtQuick\nItem { property var r: /'/; property var q: Qt['x'] }", "Qt only"),
            ("import QtQuick\nItem { property string s: `${ Qt }` }", "Qt only"),
            ("import QtQuick\nItem { property var x: \\u0074his }", "backslash"),
            ("import QtQuick\nItem { Item { anchors.fill: parent } }", "parent"),
            ("import QtQuick\nItem { property var w: Window.window }", "Window"),
        ] {
            let e = check(src).unwrap_err();
            assert!(e.contains(why), "{src}: {e}");
        }
    }
}
