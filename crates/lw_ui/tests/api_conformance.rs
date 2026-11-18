//! API-conformance gate (DX4.13): LocalWave is the reference pxui app, so its UI code must use
//! pxui only through the public call shapes of the spec section "How an APP uses pxui".
//!
//! Scans `lw_ui/src` and `lw_bin/src` with plain token matching (comments, string literals and
//! `#[cfg(test)]` items are blanked first). Every violation is `file<TAB>rule<TAB>snippet`;
//! `api-conformance-allowlist.tsv` lists the known ones, optionally followed by `<TAB>reason`
//! (required for R7). The test fails on a new violation, on a stale allowlist line and on an R7
//! line without a reason, so the allowlist only shrinks; every U package must shrink it.
//!
//! Rules:
//! - R1 struct literal of a pxui style/option type; of a pxui value type (`Node`, `Edges`, `Look`, `Rgba`, `Font`,
//!   `State`, `States`) outside a `const`/`static` item. Builders are the shape (D25.2): name a `const` for a reused style;
//!   a one-off or runtime field may be built inline (`&BUTTON.disabled(!ok)`, `Node::row().gap(8.0)`).
//! - R2 `Flags::CLICKABLE` outside a registered body fn (`fn *_body`); use `pressable`/`button`.
//! - R3 app-side shadow theme: reading the colour table / resolving tokens into colours.
//! - R4 key taken from display text (`format!`, `.to_string()`, `.as_str()` or a text-named variable).
//! - R5 per-frame allocation (`format!`, `.to_string()`, `Vec::new()`, `vec![`, `.collect::<Vec`)
//!   inside a function that takes `ui: &mut Ui`.
//! - R6 colour or look built in a build path: `ui.color(`, `paint.color(` or `Look::NONE.` inside a
//!   function that takes `ui: &mut Ui` (looks are theme consts built from `Rgba::token`).
//! - R7 `pressable` site: composite content only; each allowlist line names its reason.
//! - R8 helper fn whose body is a single pxui widget call (call the widget directly).
//!
//! To regenerate after fixing violations, run with `API_CONFORMANCE_BLESS=1` (writes the file).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const STYLE_TYPES: &[&str] = &[
    "Label", "Button", "IconButton", "Toggle", "Segmented", "Menu", "MenuItem", "Dialog",
    "DialogButton", "TextInputOptions",
];
const KEYED_CALLS: &[&str] = &[
    "button", "button_content", "pressable", "icon_button", "menu", "toggle",
    "segmented", "dialog", "node", "view_of", "id",
];
/// pxui value types whose literals belong in `const` items only (R1).
const VALUE_TYPES: &[&str] = &["Node", "Edges", "Look", "Rgba", "Font", "State", "States"];
/// pxui widget calls a one-call helper must not wrap (R8).
const WIDGETS: &[&str] = &[
    "badge", "button", "button_content", "checkbox", "dialog", "disclosure", "dropdown", "grid_list",
    "icon_button", "label", "list", "menu", "menu_bar", "modal", "panel", "picker", "popup", "pressable", "progress",
    "radio", "scrub", "search_field", "section_index", "section_list", "segmented",
    "selection_list", "slider", "spinner", "table_view", "tabs", "text_area", "text_field", "toasts", "toggle",
    "tooltip", "tree",
];
const TEXT_NAMES: &[&str] = &["label", "title", "name", "text"];
const ALLOC_TOKENS: &[&str] = &["format!(", "vec![", ".to_string()", "Vec::new()", ".collect::<Vec"];

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Blank comments and string/char literal contents (newlines and length preserved).
fn blank_text(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for k in from..to {
            if out[k] != b'\n' {
                out[k] = b' ';
            }
        }
    };
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            let end = b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |p| i + p);
            blank(&mut out, i, end);
            i = end;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let (mut depth, mut j) = (1, i + 2);
            while j < b.len() && depth > 0 {
                if b[j] == b'/' && b.get(j + 1) == Some(&b'*') {
                    depth += 1;
                    j += 2;
                } else if b[j] == b'*' && b.get(j + 1) == Some(&b'/') {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut out, i, j);
            i = j;
        } else if b[i] == b'r' && (i == 0 || !is_ident(b[i - 1])) && matches!(b.get(i + 1), Some(b'"' | b'#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while b.get(j) == Some(&b'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) != Some(&b'"') {
                i += 1;
                continue;
            }
            let start = j + 1;
            let mut end = start;
            while end < b.len() {
                if b[end] == b'"' && (0..hashes).all(|h| b.get(end + 1 + h) == Some(&b'#')) {
                    break;
                }
                end += 1;
            }
            blank(&mut out, start, end);
            i = (end + 1 + hashes).min(b.len());
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let j = j.min(b.len());
            blank(&mut out, i + 1, j);
            i = j + 1;
        } else if b[i] == b'\'' {
            // Char literal (not a lifetime): 'x' or '\n' or '\u{..}'.
            let close = if b.get(i + 1) == Some(&b'\\') {
                b[i + 2..].iter().take(10).position(|&c| c == b'\'').map(|p| i + 2 + p)
            } else if b.get(i + 2) == Some(&b'\'') {
                Some(i + 2)
            } else {
                None
            };
            if let Some(c) = close {
                blank(&mut out, i + 1, c);
                i = c + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Index just past the `}` matching the `{` at `open`.
fn block_end(b: &[u8], open: usize) -> usize {
    let mut depth = 0;
    for (k, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return k + 1;
                }
            }
            _ => {}
        }
    }
    b.len()
}

fn find_all(b: &[u8], needle: &str) -> Vec<usize> {
    let n = needle.as_bytes();
    (0..b.len().saturating_sub(n.len() - 1)).filter(|&i| &b[i..i + n.len()] == n).collect()
}

/// Blank every `#[cfg(test)]` item. Returns names of `#[cfg(test)] mod name;` declarations.
fn blank_cfg_test(b: &mut [u8]) -> Vec<String> {
    let mut test_mods = Vec::new();
    for at in find_all(b, "#[cfg(test)]") {
        if b[at] == b' ' {
            continue; // already inside a blanked region
        }
        let mut j = at + "#[cfg(test)]".len();
        let semi_or_brace = b[j..].iter().position(|&c| c == b';' || c == b'{').map_or(b.len(), |p| j + p);
        let end = if semi_or_brace < b.len() && b[semi_or_brace] == b'{' {
            block_end(b, semi_or_brace)
        } else {
            semi_or_brace + 1
        };
        let end = end.min(b.len());
        let head = String::from_utf8_lossy(&b[j..semi_or_brace.min(b.len())]).to_string();
        if let Some(rest) = head.trim().strip_prefix("mod ") {
            if semi_or_brace < b.len() && b[semi_or_brace] == b';' {
                test_mods.push(rest.trim().to_string());
            }
        }
        while j < end {
            if b[j] != b'\n' {
                b[j] = b' ';
            }
            j += 1;
        }
        for k in at..at + "#[cfg(test)]".len() {
            b[k] = b' ';
        }
    }
    test_mods
}

struct FnSpan {
    name: String,
    takes_ui: bool,
    body: (usize, usize),
}

fn fn_spans(b: &[u8]) -> Vec<FnSpan> {
    let mut spans = Vec::new();
    for at in find_all(b, "fn ") {
        if at > 0 && is_ident(b[at - 1]) {
            continue;
        }
        let mut j = at + 3;
        while b[j] == b' ' {
            j += 1;
        }
        let name_start = j;
        while j < b.len() && is_ident(b[j]) {
            j += 1;
        }
        let name = String::from_utf8_lossy(&b[name_start..j]).to_string();
        let Some(open) = b[j..].iter().position(|&c| c == b'(').map(|p| j + p) else { continue };
        let mut depth = 0;
        let mut close = open;
        for (k, &c) in b.iter().enumerate().skip(open) {
            match c {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = k;
                        break;
                    }
                }
                _ => {}
            }
        }
        let params: String = String::from_utf8_lossy(&b[open..close]).split_whitespace().collect();
        let takes_ui = params.contains("ui:&mutUi") || params.contains("ui:&mutpxui::Ui");
        let Some(stop) = b[close..].iter().position(|&c| c == b'{' || c == b';').map(|p| close + p) else {
            continue;
        };
        if b[stop] == b';' {
            continue;
        }
        spans.push(FnSpan { name, takes_ui, body: (stop, block_end(b, stop)) });
    }
    spans
}

/// First argument text of the call whose `(` is at `open`.
fn first_arg(b: &[u8], open: usize) -> String {
    let mut depth = 0;
    let mut end = b.len();
    for (k, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = k;
                    break;
                }
            }
            b',' if depth == 1 => {
                end = k;
                break;
            }
            _ => {}
        }
    }
    String::from_utf8_lossy(&b[open + 1..end]).trim().to_string()
}

fn line_of(src: &str, at: usize) -> String {
    let start = src[..at].rfind('\n').map_or(0, |p| p + 1);
    let end = src[at..].find('\n').map_or(src.len(), |p| at + p);
    src[start..end].split_whitespace().collect::<Vec<_>>().join(" ")
}

fn scan(rel: &str, src: &str, out: &mut Vec<(String, &'static str, String)>) {
    let mut code = blank_text(src);
    blank_cfg_test(&mut code);
    let fns = fn_spans(&code);
    let mut hit = |rule: &'static str, at: usize| out.push((rel.to_string(), rule, line_of(src, at)));
    let innermost = |at: usize| {
        fns.iter().filter(|f| f.body.0 <= at && at < f.body.1).min_by_key(|f| f.body.1 - f.body.0)
    };

    // R1: `Type {` literals (not item headers, return types or impls); value types only outside consts.
    let consts = const_spans(&code);
    for ty in STYLE_TYPES.iter().chain(VALUE_TYPES) {
        for at in find_all(&code, ty) {
            let end = at + ty.len();
            if (at > 0 && is_ident(code[at - 1])) || code.get(end).is_some_and(|&c| is_ident(c)) {
                continue;
            }
            let mut j = end;
            while code.get(j).is_some_and(|c| c.is_ascii_whitespace()) {
                j += 1;
            }
            if code.get(j) != Some(&b'{') {
                continue;
            }
            let before = String::from_utf8_lossy(&code[..at]).to_string();
            let before = before.trim_end().trim_end_matches("pxui::").trim_end();
            if ["->", "impl", "for", "struct", "enum", "trait", "mod", "type", "as", "dyn"]
                .iter()
                .any(|kw| before.ends_with(kw))
            {
                continue;
            }
            // `Enum::Variant {` (an enum's variant named like a pxui type, `Column::State`) is no literal.
            let seg = code[..at].strip_suffix(b"::").map(|p| &p[p.iter().rposition(|&c| !is_ident(c)).map_or(0, |k| k + 1)..]);
            if seg.is_some_and(|seg| seg.first().is_some_and(u8::is_ascii_uppercase)) {
                continue;
            }
            if VALUE_TYPES.contains(ty) && consts.iter().any(|&(a, b)| a <= at && at < b) {
                continue;
            }
            hit("R1", at);
        }
    }

    // R2: hand-built clickables outside body fns.
    for at in find_all(&code, "Flags::CLICKABLE") {
        if !innermost(at).is_some_and(|f| f.name.ends_with("_body")) {
            hit("R2", at);
        }
    }

    // R3: shadow theme.
    for at in find_all(&code, "COLORS[") {
        hit("R3", at);
    }
    for at in find_all(&code, "fn resolve(") {
        let sig_end = code[at..].iter().position(|&c| c == b'{' || c == b';').map_or(code.len(), |p| at + p);
        let sig = String::from_utf8_lossy(&code[at..sig_end]);
        if sig.contains("ColorToken") || sig.contains("Rgba") {
            hit("R3", at);
        }
    }

    // R4: keys from display text.
    for call in KEYED_CALLS {
        for at in find_all(&code, call) {
            let end = at + call.len();
            if (at > 0 && is_ident(code[at - 1])) || code.get(end) != Some(&b'(') {
                continue;
            }
            if String::from_utf8_lossy(&code[at.saturating_sub(3)..at]) == "fn " {
                continue;
            }
            let arg = first_arg(&code, end);
            let compact: String = arg.split_whitespace().collect();
            let bare = compact.trim_start_matches('&');
            let text_var = TEXT_NAMES.contains(&bare);
            if text_var || compact.contains("format!(") || compact.contains(".to_string()") || compact.contains(".as_str()") {
                hit("R4", at);
            }
        }
    }

    // R6: colours and looks built in build paths.
    for tok in ["ui.color(", "paint.color(", "Look::NONE."] {
        for at in find_all(&code, tok) {
            if (at > 0 && is_ident(code[at - 1])) || !innermost(at).is_some_and(|f| f.takes_ui) {
                continue;
            }
            hit("R6", at);
        }
    }

    // R7: pressable sites (composite content; the allowlist names each reason).
    for at in find_all(&code, ".pressable(") {
        hit("R7", at);
    }

    // R8: a helper whose whole body is one widget call.
    for f in &fns {
        if !f.name.ends_with("_body") && single_widget_call(&code[f.body.0 + 1..f.body.1 - 1]) {
            hit("R8", f.body.0);
        }
    }

    // R5: per-frame allocation in ui build fns (innermost fn decides, closures count as inside).
    for tok in ALLOC_TOKENS {
        for at in find_all(&code, tok) {
            if innermost(at).is_some_and(|f| f.takes_ui) {
                hit("R5", at);
            }
        }
    }
}

/// Spans of `const`/`static` items (`const NAME: T = ..;`), not `const fn`.
fn const_spans(b: &[u8]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for kw in ["const ", "static "] {
        for at in find_all(b, kw) {
            if (at > 0 && is_ident(b[at - 1])) || b[at + kw.len()..].starts_with(b"fn ") {
                continue;
            }
            let Some(eq) = b[at..].iter().position(|&c| c == b'=' || c == b';' || c == b'{').map(|p| at + p) else { continue };
            if b[eq] != b'=' {
                continue;
            }
            let mut depth = 0i32;
            let end = b[eq..]
                .iter()
                .position(|&c| {
                    match c {
                        b'(' | b'[' | b'{' => depth += 1,
                        b')' | b']' | b'}' => depth -= 1,
                        _ => {}
                    }
                    c == b';' && depth == 0
                })
                .map_or(b.len(), |p| eq + p);
            spans.push((at, end));
        }
    }
    spans
}

/// Index of the `)` matching the `(` at `open`.
fn paren_end(b: &[u8], open: usize) -> usize {
    let mut depth = 0;
    for (k, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return k;
                }
            }
            _ => {}
        }
    }
    b.len()
}

/// True when `body` (a fn body without its braces) is one `ui.<widget>(..)` call, optionally followed by
/// field/method accesses (`.clicked`, `.is_some()`) and a `;`.
fn single_widget_call(body: &[u8]) -> bool {
    let text = String::from_utf8_lossy(body);
    let t = text.trim();
    let Some(rest) = t.strip_prefix("ui.") else { return false };
    let name_len = rest.bytes().take_while(|&c| is_ident(c)).count();
    if !WIDGETS.contains(&&rest[..name_len]) || !rest[name_len..].starts_with('(') {
        return false;
    }
    let bytes = rest.as_bytes();
    let close = paren_end(bytes, name_len);
    let tail: String = rest[(close + 1).min(rest.len())..].split_whitespace().collect();
    let tail = tail.strip_suffix(';').unwrap_or(&tail);
    tail.split('.').skip(1).all(|part| {
        let part = part.strip_suffix("()").unwrap_or(part);
        !part.is_empty() && part.bytes().all(is_ident)
    }) && (tail.is_empty() || tail.starts_with('.'))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn is_test_file(path: &Path, test_mods: &BTreeMap<PathBuf, Vec<String>>) -> bool {
    let stem = path.file_stem().unwrap().to_string_lossy();
    if stem == "tests" || stem.ends_with("_tests") || path.components().any(|c| c.as_os_str() == "tests") {
        return true;
    }
    let dir = if stem == "mod" { path.parent().unwrap().parent() } else { path.parent() };
    let name = if stem == "mod" {
        path.parent().unwrap().file_name().unwrap().to_string_lossy().to_string()
    } else {
        stem.to_string()
    };
    // A parent file `<dir>/<parent>.rs` or `<dir>/mod.rs`/`lib.rs` declared it `#[cfg(test)] mod name;`.
    test_mods.iter().any(|(decl, names)| {
        names.contains(&name)
            && dir.is_some_and(|d| {
                let decl_dir = decl.parent().unwrap();
                let decl_stem = decl.file_stem().unwrap().to_string_lossy();
                if ["lib", "main", "mod"].contains(&&*decl_stem) { d == decl_dir } else { d == decl_dir.join(&*decl_stem) }
            })
    })
}

#[test]
fn api_conformance_matches_allowlist() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    rust_files(&crates.join("lw_bin/src"), &mut files);

    let mut sources = BTreeMap::new();
    let mut test_mods = BTreeMap::new();
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        let mut code = blank_text(&src);
        test_mods.insert(path.clone(), blank_cfg_test(&mut code));
        sources.insert(path.clone(), src);
    }

    let mut found = Vec::new();
    for (path, src) in &sources {
        if is_test_file(path, &test_mods) {
            continue;
        }
        let rel = path.strip_prefix(crates).unwrap().to_string_lossy().replace('\\', "/");
        scan(&rel, src, &mut found);
    }
    let mut actual: BTreeMap<String, usize> = BTreeMap::new();
    for (file, rule, snippet) in &found {
        *actual.entry(format!("{file}\t{rule}\t{snippet}")).or_default() += 1;
    }

    let list_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/api-conformance-allowlist.tsv");
    if std::env::var_os("API_CONFORMANCE_BLESS").is_some() {
        // Kept lines keep their reasons, in order; new lines get none (R7 then fails until one is written).
        let old = fs::read_to_string(&list_path).unwrap_or_default();
        let mut reasons: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for line in old.lines() {
            let mut cols = line.splitn(4, '\t');
            let key = [cols.next(), cols.next(), cols.next()].map(|c| c.unwrap_or("")).join("\t");
            reasons.entry(key).or_default().push(cols.next().unwrap_or("").to_string());
        }
        let mut text = String::new();
        for (line, n) in &actual {
            let kept = reasons.get(line).map_or(&[][..], |r| &r[..]);
            for k in 0..*n {
                text.push_str(line);
                if let Some(reason) = kept.get(k).filter(|r| !r.is_empty()) {
                    text.push('\t');
                    text.push_str(reason);
                }
                text.push('\n');
            }
        }
        fs::write(&list_path, text).unwrap();
        return;
    }
    let list = fs::read_to_string(&list_path).unwrap_or_default();
    let mut allowed: BTreeMap<String, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    for line in list.lines().filter(|l| !l.trim().is_empty()) {
        let mut cols = line.splitn(4, '\t');
        let (file, rule, snippet, reason) = (cols.next().unwrap_or(""), cols.next().unwrap_or(""), cols.next().unwrap_or(""), cols.next().unwrap_or(""));
        if rule == "R7" && reason.trim().is_empty() {
            problems.push(format!("R7 allowlist line without a reason: {line}"));
        }
        *allowed.entry(format!("{file}\t{rule}\t{snippet}")).or_default() += 1;
    }

    for (line, n) in &actual {
        let known = allowed.get(line).copied().unwrap_or(0);
        if *n > known {
            problems.push(format!("new violation (x{}): {line}", n - known));
        }
    }
    for (line, n) in &allowed {
        let now = actual.get(line).copied().unwrap_or(0);
        if now < *n {
            problems.push(format!("stale allowlist line (x{}): {line}", n - now));
        }
    }
    assert!(
        problems.is_empty(),
        "API conformance drift (see tests/api_conformance.rs; the allowlist only shrinks):\n{}",
        problems.join("\n")
    );
}

#[test]
fn rule_helpers_recognise_their_shapes() {
    assert!(single_widget_call(b" ui.button(\"k\", \"Go\", &B).clicked "));
    assert!(single_widget_call(b"ui.toggle(key, text, on, &T);"));
    assert!(!single_widget_call(b"let r = ui.button(k, t, &B); r.clicked"));
    assert!(!single_widget_call(b"ui.node(k, n)"));
    let src = b"const A: Node = Node { w: 1 }; fn f() { let x = Node { w: 2 }; }";
    let spans = const_spans(src);
    let first = find_all(src, "Node {")[0];
    let second = find_all(src, "Node {")[1];
    assert!(spans.iter().any(|&(a, b)| a <= first && first < b));
    assert!(!spans.iter().any(|&(a, b)| a <= second && second < b));
}
