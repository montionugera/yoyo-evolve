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
//! Limits, stated rather than implied: the tool's own `(N matches)` /
//! `(N files)` footer and its `details` JSON are not rewritten (the withheld
//! line carries the truth); `find` warning lines are kept verbatim even if they
//! name a denied path; and `rg` still runs without `--no-config`.

/// Tools whose `path` argument is a root they walk below, so their output can
/// name files the argument check never saw.
const RECURSIVE_READERS: &[&str] = &["search", "list_files"];

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
#[cfg(test)]
fn search_result_path(line: &str) -> Option<&str> {
    search_path_candidates(line).into_iter().next()
}

/// The in-band note appended when `count > 0`. Glyph-free on purpose.
fn withheld_note(count: usize) -> String {
    let noun = if count == 1 { "result" } else { "results" };
    format!(
        "[{count} {noun} withheld: inside a directory the fence refuses (--deny-dir / --allow-dir / [directories])]"
    )
}

/// Filter one tool's text output. Returns the kept text and how many lines were
/// withheld; with `0` withheld the input comes back byte-identical.
fn filter_fenced_lines(
    tool: &str,
    output: &str,
    refused: &dyn Fn(&str) -> bool,
) -> (String, usize) {
    if !RECURSIVE_READERS.contains(&tool) {
        return (output.to_string(), 0);
    }
    let mut kept: Vec<&str> = Vec::new();
    let mut withheld = 0usize;
    for line in output.split('\n') {
        let deny = match tool {
            "list_files" => {
                let p = line.trim_end_matches('\r');
                !p.is_empty() && refused(p)
            }
            _ => search_path_candidates(line).into_iter().any(refused),
        };
        if deny {
            withheld += 1;
        } else {
            kept.push(line);
        }
    }
    if withheld == 0 {
        return (output.to_string(), 0);
    }
    let mut text = kept.join("\n");
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&withheld_note(withheld));
    (text, withheld)
}

/// Apply the filter to every text block of a successful tool result. Non-text
/// blocks and the `details` JSON pass through untouched; tools other than
/// `search` / `list_files` are returned unchanged.
pub(crate) fn fence_tool_result(
    tool: &str,
    mut result: yoagent::types::ToolResult,
    refused: &dyn Fn(&str) -> bool,
) -> yoagent::types::ToolResult {
    if !RECURSIVE_READERS.contains(&tool) {
        return result;
    }
    for block in result.content.iter_mut() {
        if let yoagent::types::Content::Text { text } = block {
            let (filtered, n) = filter_fenced_lines(tool, text, refused);
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
        let (out, n) = filter_fenced_lines("search", &output, &refused);
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
        let (out, n) = filter_fenced_lines("list_files", &output, &refused);
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
            let (out, n) = filter_fenced_lines(tool, &output, &refused);
            assert_eq!(n, 0);
            assert_eq!(out, output);
        }
    }

    #[test]
    fn other_tools_are_never_filtered() {
        let (_dir, root, r) = fixture();
        let refused = |p: &str| r.check_path(p).is_err();
        let output = format!("{root}/secrets/key.txt:1:token={CANARY}");
        let (out, n) = filter_fenced_lines("read_file", &output, &refused);
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
        let (out, n) = filter_fenced_lines("search", &line, &refused);
        assert_eq!(n, 1, "{out}");
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
        let fenced = fence_tool_result("search", mk(), &refused);
        assert!(!get(&fenced).contains(CANARY));
        let untouched = fence_tool_result("bash", mk(), &refused);
        assert_eq!(get(&untouched), text);
    }
}
