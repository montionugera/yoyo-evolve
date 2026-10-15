//! Fence the RESULTS of a recursive reader, not just its path argument (#1022).
//!
//! `GuardedTool` / `ArcGuardedTool` check a tool's `path` argument against the
//! directory fence (`--allow-dir` / `--deny-dir` / `[directories]`, plus the
//! `/cd`-added deny list). For `read_file` that argument IS the file read. For
//! `search` and `list_files` it is a ROOT: `search CANARY .` passed the check on
//! `.` and then the inner yoagent tool walked straight into `./secrets/` and
//! handed the model `./secrets/key.txt:1:token=CANARY`. This module filters the
//! text those two tools return, using the caller's own refusal predicate, so
//! there is one predicate for the argument and the results, not two.
//!
//! **Byte-identical when nothing is withheld** — that is every user without a
//! fence and every fenced search that stayed outside the denied subtree, i.e.
//! the whole regression surface. When something is withheld, one glyph-free
//! line says how many, so the model learns results were removed rather than
//! concluding they never existed.
//!
//! **Fail closed on lines it cannot attribute (#1023).** `search` is yoagent's
//! tool, which spawns `rg` itself, so yoyo does not build its argv and cannot
//! add `--no-config`; `rg` reads whatever `RIPGREP_CONFIG_PATH` names. A config
//! can change the OUTPUT SHAPE this filter reads a path from — measured on
//! ripgrep 14.1.0: `--no-filename` printed the denied file's line as a bare
//! `1:TOKEN`, `--null` put a NUL where the `:` was, `--json` printed JSON
//! objects, `--path-separator=X` printed `XtmpX…` — and every one of those lines
//! used to pass as "not a result line". So while a fence is active and the root
//! is a directory, a line before yoagent's own footer is kept only if its first
//! `path:N:` candidate names a path under the searched root (and holds no NUL).
//! Anything else is dropped and counted in its own note. The property is
//! "a user's ripgrep config never shapes what a fenced search returns", decided
//! by where a line's path came from — no list of rg flags is consulted.
//!
//! Limits, stated rather than implied: the tool's own `(N matches)` /
//! `(N files)` footer and its `details` JSON are not rewritten (the withheld
//! line carries the truth); `find` and `rg` warning lines after the footer are
//! kept verbatim even if they name a denied path. A single-FILE root is not put
//! in fail-closed mode: `rg` prints that file's lines without a path, and the
//! root itself already passed the argument check.

/// Tools whose `path` argument is a root they walk below, so their output can
/// name files the argument check never saw.
const RECURSIVE_READERS: &[&str] = &["search", "list_files"];

/// The whole text yoagent's `search` returns when the subprocess printed
/// nothing; such output cannot carry a result line.
const NO_MATCHES_PREFIX: &str = "No matches found for '";

/// Every candidate file path a `search` result line could start with: the
/// prefix before each `:<digits>:` occurrence, first one first. A line with no
/// such occurrence (a header, the `(N matches)` footer, a blank, a single-file
/// search's bare `N:text`) yields nothing and is kept by the filter.
///
/// All splits are returned, not just the first, because a directory or file
/// name may itself contain `:<digits>:`; the filter withholds a line if ANY
/// candidate is refused, so an odd name fails closed rather than leaking.
fn search_path_candidates(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for (i, _) in line.match_indices(':') {
        let rest = &line[i + 1..];
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        // `digits` counts ASCII chars, so it is also a byte length here.
        if i > 0 && digits > 0 && rest[digits..].starts_with(':') {
            out.push(&line[..i]);
        }
    }
    out
}

/// The path a `search` result line names: the prefix before its FIRST
/// `:<digits>:`. `None` when the line is not a result line.
fn search_result_path(line: &str) -> Option<&str> {
    search_path_candidates(line).into_iter().next()
}

/// Whether a `search` line can be attributed to a file under `root`: its first
/// `path:N:` candidate exists, starts with the root exactly as it was handed to
/// the subprocess (that is how both `rg` and `grep -r` print it), and holds no
/// NUL (no path does). A reshaped line fails this, whatever reshaped it.
fn attributable_to_root(line: &str, root: &str) -> bool {
    search_result_path(line).is_some_and(|p| p.starts_with(root) && !p.contains('\0'))
}

/// One of the lines yoagent's `search` appends after the matches. Everything
/// from the LAST such line on is the tool's own tail (footer, then any
/// `Warnings:` block); picking the last one means a footer-shaped line earlier
/// in the output can only cause more dropping, never less.
fn is_search_footer(line: &str) -> bool {
    if let Some(n) = line
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(" matches)"))
    {
        return !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit());
    }
    line.starts_with("... (showing the first ")
        && line.ends_with(" matches; there are more. Narrow the pattern, path or include.)")
}

/// The in-band note appended when `count > 0`. Glyph-free on purpose.
fn withheld_note(count: usize) -> String {
    let noun = if count == 1 { "result" } else { "results" };
    format!(
        "[{count} {noun} withheld: inside a directory the fence refuses (--deny-dir / --allow-dir / [directories])]"
    )
}

/// The note for lines dropped because no file could be named for them.
fn unattributed_note(count: usize) -> String {
    let noun = if count == 1 { "line" } else { "lines" };
    format!(
        "[{count} {noun} withheld: a directory fence is active and the line does not name a file under the searched path in the usual path:line: form (a ripgrep config via RIPGREP_CONFIG_PATH can reshape output), so it was dropped rather than risk a leak]"
    )
}

/// Filter one tool's text output. Returns the kept text and how many lines were
/// withheld; with `0` withheld the input comes back byte-identical.
///
/// `strict_root` is `Some(root)` only while a fence is active and a `search`
/// walked the directory `root`: then a line before the tool's footer that is
/// not [`attributable_to_root`] is dropped too (#1023). `None` keeps the
/// pre-#1023 behaviour exactly.
fn filter_fenced_lines(
    tool: &str,
    output: &str,
    refused: &dyn Fn(&str) -> bool,
    strict_root: Option<&str>,
) -> (String, usize) {
    if !RECURSIVE_READERS.contains(&tool) {
        return (output.to_string(), 0);
    }
    let lines: Vec<&str> = output.split('\n').collect();
    // Lines at or after `tail_start` are yoagent's own; only the body before it
    // must be attributable. With no footer the whole output is body, unless it
    // is the tool's "No matches" message, which carries no subprocess line.
    let strict_root = match strict_root {
        Some(_) if tool != "search" || output.starts_with(NO_MATCHES_PREFIX) => None,
        other => other,
    };
    let tail_start = lines
        .iter()
        .rposition(|l| is_search_footer(l))
        .unwrap_or(lines.len());
    let mut kept: Vec<&str> = Vec::new();
    let mut denied = 0usize;
    let mut unattributed = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let deny = match tool {
            "list_files" => {
                let p = line.trim_end_matches('\r');
                !p.is_empty() && refused(p)
            }
            _ => search_path_candidates(line).into_iter().any(refused),
        };
        if deny {
            denied += 1;
        } else if strict_root
            .is_some_and(|root| idx < tail_start && !attributable_to_root(line, root))
        {
            unattributed += 1;
        } else {
            kept.push(line);
        }
    }
    let withheld = denied + unattributed;
    if withheld == 0 {
        return (output.to_string(), 0);
    }
    let mut text = kept.join("\n");
    for note in [
        (denied > 0).then(|| withheld_note(denied)),
        (unattributed > 0).then(|| unattributed_note(unattributed)),
    ]
    .into_iter()
    .flatten()
    {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&note);
    }
    (text, withheld)
}

/// Apply the filter to every text block of a successful tool result. Non-text
/// blocks and the `details` JSON pass through untouched; tools other than
/// `search` / `list_files` are returned unchanged.
///
/// `fence_active` must be the UNION of every fence source (startup restrictions
/// and the `/cd` lists), and `root` the `path` the tool walked (`.` when the
/// argument was absent, as yoagent defaults it). Fail-closed mode is decided
/// here, inside the one function every guarded search result passes through,
/// so no caller can forget it.
pub(crate) fn fence_tool_result(
    tool: &str,
    mut result: yoagent::types::ToolResult,
    refused: &dyn Fn(&str) -> bool,
    fence_active: bool,
    root: &str,
) -> yoagent::types::ToolResult {
    if !RECURSIVE_READERS.contains(&tool) {
        return result;
    }
    let strict_root = (fence_active && !std::path::Path::new(root).is_file()).then_some(root);
    for block in result.content.iter_mut() {
        if let yoagent::types::Content::Text { text } = block {
            let (filtered, n) = filter_fenced_lines(tool, text, refused, strict_root);
            if n > 0 {
                *text = filtered;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryRestrictions;

    const CANARY: &str = "CANARY_Q7Z";

    /// A tempdir holding `secrets/key.txt` (the canary) and `src/a.txt`, plus a
    /// predicate refusing everything under `<tmp>/secrets` via the REAL
    /// `DirectoryRestrictions::check_path` (absolute paths, so cwd is not read).
    fn fixture() -> (tempfile::TempDir, String, DirectoryRestrictions) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        std::fs::create_dir_all(dir.path().join("secrets")).unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("secrets/key.txt"),
            format!("token={CANARY}\n"),
        )
        .unwrap();
        std::fs::write(dir.path().join("src/a.txt"), format!("ok {CANARY}\n")).unwrap();
        let r = DirectoryRestrictions {
            allow: vec![],
            deny: vec![format!("{root}/secrets")],
        };
        (dir, root, r)
    }

    #[test]
    fn search_result_naming_a_denied_file_is_withheld_and_sibling_survives() {
        let (dir, root, r) = fixture();
        // Anti-vacuous: the canary really is on disk inside the denied dir.
        let on_disk = std::fs::read_to_string(dir.path().join("secrets/key.txt")).unwrap();
        assert!(on_disk.contains(CANARY));
        let sibling = format!("{root}/src/a.txt:1:ok {CANARY}");
        let output = format!("{root}/secrets/key.txt:1:token={CANARY}\n{sibling}\n(2 matches)");
        let refused = |p: &str| r.check_path(p).is_err();
        let (out, n) = filter_fenced_lines("search", &output, &refused, None);
        assert_eq!(n, 1);
        assert!(!out.contains("token="), "canary leaked: {out}");
        assert!(
            !out.contains("secrets/key.txt"),
            "denied path leaked: {out}"
        );
        assert!(out.contains(&sibling), "allowed sibling lost: {out}");
        assert!(out.contains("[1 result withheld:"), "note missing: {out}");
    }

    #[test]
    fn list_files_entry_inside_denied_dir_is_withheld() {
        let (_dir, root, r) = fixture();
        let output = format!("{root}/secrets/key.txt\n{root}/src/a.txt\n\n(2 files)");
        let refused = |p: &str| r.check_path(p).is_err();
        let (out, n) = filter_fenced_lines("list_files", &output, &refused, None);
        assert_eq!(n, 1);
        assert!(!out.contains("secrets/key.txt"), "{out}");
        assert!(out.contains(&format!("{root}/src/a.txt")), "{out}");
        assert!(out.ends_with(&withheld_note(1)), "{out}");
    }

    #[test]
    fn output_with_nothing_denied_is_byte_identical() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        for (tool, output) in [
            (
                "search",
                format!("{root}/src/a.txt:1:ok {CANARY}\n(1 matches)"),
            ),
            ("list_files", format!("{root}/src/a.txt\n\n(1 files)")),
            ("search", "No matches found for 'x'".to_string()),
        ] {
            let (out, n) = filter_fenced_lines(tool, &output, &refused, None);
            assert_eq!(n, 0);
            assert_eq!(out, output);
        }
    }

    #[test]
    fn other_tools_are_never_filtered() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        let output = format!("{root}/secrets/key.txt:1:token={CANARY}");
        let (out, n) = filter_fenced_lines("read_file", &output, &refused, None);
        assert_eq!((out.as_str(), n), (output.as_str(), 0));
    }

    #[test]
    fn search_path_splits_on_the_first_colon_digits_colon() {
        assert_eq!(
            search_result_path("src/a.rs:3:let x = \"b:12:c\""),
            Some("src/a.rs")
        );
        assert_eq!(search_result_path("(3 matches)"), None);
        assert_eq!(search_result_path("12:bare single-file hit"), None);
        assert_eq!(search_result_path("a:b:c"), None);
        assert_eq!(search_result_path(""), None);
        // Multi-byte text must not panic (no byte-indexing past a boundary).
        assert_eq!(search_result_path("ü/✓.rs:7:✓:9:✓"), Some("ü/✓.rs"));
    }

    #[test]
    fn a_name_containing_colon_digits_fails_closed() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        // First split lands at `weird` (allowed); a later split names the
        // denied file — the line is withheld rather than leaked.
        let line = format!("{root}/weird:1:x/../secrets/key.txt:4:token");
        let (out, n) = filter_fenced_lines("search", &line, &refused, None);
        assert_eq!(n, 1, "{out}");
    }

    /// Bodies `search` returned through the real guarded yoagent tool with
    /// `RIPGREP_CONFIG_PATH` naming a one-flag config (ripgrep 14.1.0, Day 229
    /// probe, `{R}` = the searched root, `secrets/` denied). Every row named
    /// the denied file or carried its line, and every row passed the
    /// pre-#1023 filter untouched.
    fn reshaped_bodies(root: &str) -> Vec<(&'static str, String)> {
        vec![
            ("--no-filename", format!("1:token={CANARY}\n1:ok {CANARY}\n(2 matches)")),
            (
                "--null",
                format!("{root}/secrets/key.txt\u{0}1:token={CANARY}\n{root}/src/a.txt\u{0}1:ok {CANARY}\n(2 matches)"),
            ),
            (
                "--json",
                format!("{{\"type\":\"match\",\"data\":{{\"path\":{{\"text\":\"{root}/secrets/key.txt\"}},\"lines\":{{\"text\":\"token={CANARY}\\n\"}},\"line_number\":1}}}}\n(1 matches)"),
            ),
            (
                "--path-separator=X",
                format!("{}Xsecrets/key.txt:1:token={CANARY}\n(1 matches)", root.replace('/', "X")),
            ),
            (
                "-C1 context line",
                format!("{root}/secrets/key.txt-2-token={CANARY}\n--\n{root}/src/a.txt:1:ok {CANARY}\n(2 matches)"),
            ),
        ]
    }

    #[test]
    fn unattributable_lines_under_an_active_fence_are_dropped() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        for (flag, body) in reshaped_bodies(&root) {
            // Anti-vacuous: the fixture really carries the denied file's line.
            assert!(body.contains(&format!("token={CANARY}")), "{flag}");
            let (out, n) = filter_fenced_lines("search", &body, &refused, Some(&root));
            assert!(n > 0, "{flag}: nothing withheld: {out}");
            assert!(!out.contains("token="), "{flag}: canary leaked: {out}");
            assert!(
                !out.contains("secrets"),
                "{flag}: denied path leaked: {out}"
            );
            assert!(out.contains("withheld"), "{flag}: no note: {out}");
        }
    }

    #[test]
    fn reshaped_bodies_without_a_fence_are_byte_identical() {
        let (_dir, root, _r) = fixture();
        // No fence: nothing is refused and strict mode is off.
        let refused = |_: &str| false;
        for (flag, body) in reshaped_bodies(&root) {
            let (out, n) = filter_fenced_lines("search", &body, &refused, None);
            assert_eq!((out.as_str(), n), (body.as_str(), 0), "{flag}");
        }
    }

    #[test]
    fn ordinary_results_under_an_active_fence_pass_byte_identically() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        for (root_arg, body) in [
            (
                root.clone(),
                format!("{root}/src/a.txt:1:ok {CANARY}\n{root}/src/a.txt:2:x:7:y\n(2 matches)"),
            ),
            (
                root.clone(),
                format!("{root}/src/a.txt:1:ok\n... (showing the first 1 matches; there are more. Narrow the pattern, path or include.)\nWarnings:\nrg: {root}/src/x: Permission denied (os error 13)"),
            ),
            (".".to_string(), "./src/a.txt:1:ok\n(1 matches)".to_string()),
            (root.clone(), format!("No matches found for '{CANARY}'")),
        ] {
            let (out, n) = filter_fenced_lines("search", &body, &refused, Some(&root_arg));
            assert_eq!((out.as_str(), n), (body.as_str(), 0));
        }
    }

    #[test]
    fn a_single_file_root_is_not_put_in_fail_closed_mode() {
        let (dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        let file = dir.path().join("src/a.txt").to_string_lossy().to_string();
        let text = format!("1:ok {CANARY}\n(1 matches)");
        let mk = || yoagent::types::ToolResult {
            content: vec![yoagent::types::Content::Text { text: text.clone() }],
            details: serde_json::json!({}),
        };
        assert_eq!(
            text_of(&fence_tool_result("search", mk(), &refused, true, &file)),
            text
        );
        // Near-miss: the same bare line under a DIRECTORY root is dropped.
        let dropped = text_of(&fence_tool_result("search", mk(), &refused, true, &root));
        assert!(!dropped.contains(CANARY), "{dropped}");
        assert!(dropped.contains("[1 line withheld:"), "{dropped}");
    }

    fn text_of(res: &yoagent::types::ToolResult) -> String {
        res.content
            .iter()
            .filter_map(|b| match b {
                yoagent::types::Content::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// End to end through the REAL wrappers and the REAL yoagent tools: the
    /// argument (the tempdir root) is allowed, the walk reaches `secrets/`, and
    /// the result the model would receive must not carry the canary. Both the
    /// Box path (main agent) and the Arc path (sub-agents) are driven.
    #[tokio::test]
    async fn real_guarded_search_and_list_files_withhold_the_denied_subtree() {
        use yoagent::types::ToolContext;
        let (_dir, root, r) = fixture();
        let search = crate::tool_wrappers::maybe_guard(
            Box::new(yoagent::tools::search::SearchTool::default()),
            &r,
        );
        let res = search
            .execute(
                serde_json::json!({"pattern": CANARY, "path": root}),
                ToolContext::new("c1", "search"),
            )
            .await
            .expect("search over an allowed root succeeds");
        let out = text_of(&res);
        assert!(!out.contains("token="), "canary leaked via search: {out}");
        assert!(out.contains("src/a.txt"), "allowed hit lost: {out}");
        assert!(out.contains("[1 result withheld:"), "{out}");
        // The fence is active here, so fail-closed mode is on: yoagent's own
        // footer must still be recognised, or this note would appear (#1023).
        assert!(
            !out.contains("line withheld"),
            "footer not recognised: {out}"
        );
        assert!(
            !out.contains("lines withheld"),
            "footer not recognised: {out}"
        );

        let list = crate::tool_wrappers::maybe_guard_arc(
            std::sync::Arc::new(yoagent::tools::list::ListFilesTool::default()),
            &r,
        );
        let res = list
            .execute(
                serde_json::json!({"path": root}),
                ToolContext::new("c2", "list_files"),
            )
            .await
            .expect("list_files over an allowed root succeeds");
        let out = text_of(&res);
        assert!(
            !out.contains("secrets/key.txt"),
            "listed denied file: {out}"
        );
        assert!(out.contains("src/a.txt"), "{out}");
        assert!(out.contains("[1 result withheld:"), "{out}");
    }

    #[test]
    fn fence_tool_result_rewrites_text_blocks_of_search_only() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        let text = format!("{root}/secrets/key.txt:1:token={CANARY}\n(1 matches)");
        let mk = || yoagent::types::ToolResult {
            content: vec![yoagent::types::Content::Text { text: text.clone() }],
            details: serde_json::json!({}),
        };
        let get = |res: &yoagent::types::ToolResult| match &res.content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("not text"),
        };
        let fenced = fence_tool_result("search", mk(), &refused, false, &root);
        assert!(!get(&fenced).contains(CANARY));
        let untouched = fence_tool_result("bash", mk(), &refused, true, &root);
        assert_eq!(get(&untouched), text);
    }
}
