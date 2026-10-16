//! Syntax checks for the config file that `parse_config_file` cannot report (#1034).
//!
//! `config::parse_config_file` is a lenient, hand-rolled line parser with no
//! error channel: on mismatched quotes it keeps the raw value (stray quote and
//! all), and an unclosed `[section` header is silently skipped. So
//! `model = "claude-haiku-5-5` (no closing quote) made `yoyo config get model`
//! print the value WITH the quote and exit 0, and `-p` then sent that id and got
//! a 404, while nothing anywhere said the file was malformed.
//!
//! This module is deliberately **detection only**. What values parse to is
//! unchanged, byte-for-byte; changing that is a separate decision. The detector
//! is pure and table-tested here; `config::load_config_file` calls it on the
//! text of the file that won the search and prints the warning once per run.
//!
//! What it checks, and only this:
//! - a `key = value` line whose value *starts* with a quote, array or inline
//!   table and leaves a `"` or `'` string open at end of line. Bare values
//!   (`notify_command = echo it's done`) are not scanned: the parser accepts
//!   them as-is and calling an apostrophe there an "unterminated quote" would
//!   be a false alarm. Multi-line strings (`"""` / `'''`) are skipped too —
//!   the parser does not support them, but they are valid TOML and a
//!   "unterminated quote" claim about one would be wrong.
//! - a line starting with `[` that has no `]` anywhere (an unclosed header).
//!
//! Lines with no `=` that are not headers (e.g. continuation lines of a
//! multi-line array, which `parse_config_file` ignores) are never examined.

/// Kind string for an unterminated `"` / `'` string value.
pub(crate) const UNTERMINATED_QUOTE: &str = "unterminated quote";
/// Kind string for a `[section` header with no closing `]`.
pub(crate) const UNCLOSED_HEADER: &str = "unclosed [section] header";

/// Every syntax problem in `text`, as `(1-based line number, kind)`.
/// Empty when the file has none of the shapes this module checks.
pub(crate) fn config_syntax_problems(text: &str) -> Vec<(usize, &'static str)> {
    let mut problems = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if !line.contains(']') {
                problems.push((idx + 1, UNCLOSED_HEADER));
            }
            continue;
        }
        let Some((_, value)) = line.split_once('=') else {
            continue;
        };
        if value_leaves_string_open(value.trim()) {
            problems.push((idx + 1, UNTERMINATED_QUOTE));
        }
    }
    problems
}

/// Does `value` (already trimmed) open a string it never closes on this line?
fn value_leaves_string_open(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    if !matches!(first, '"' | '\'' | '[' | '{') {
        return false;
    }
    if value.starts_with("\"\"\"") || value.starts_with("'''") {
        return false;
    }
    #[derive(PartialEq)]
    enum State {
        Outside,
        Basic,
        Literal,
    }
    let mut state = State::Outside;
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match state {
            State::Outside => match c {
                '#' => break,
                '"' => state = State::Basic,
                '\'' => state = State::Literal,
                _ => {}
            },
            State::Basic => match c {
                '\\' => {
                    chars.next();
                }
                '"' => state = State::Outside,
                _ => {}
            },
            State::Literal => {
                if c == '\'' {
                    state = State::Outside;
                }
            }
        }
    }
    state != State::Outside
}

/// The warning text for `problems` found in the config file at `path`, or
/// `None` when there are none (the common case, which must print nothing).
/// The caller adds the `warning:` prefix, like the #1028 numeric warning.
/// `path` comes from the filesystem (a repo-authored name for a project-local
/// file), so it goes through `cli::sanitize_for_display` before display.
pub(crate) fn config_syntax_warning(
    path: &str,
    problems: &[(usize, &'static str)],
) -> Option<String> {
    if problems.is_empty() {
        return None;
    }
    let path = crate::cli::sanitize_for_display(path);
    let locations: Vec<String> = problems
        .iter()
        .map(|(line, kind)| format!("{path}:{line}: {kind}"))
        .collect();
    let noun = if problems.len() == 1 {
        "problem"
    } else {
        "problems"
    };
    Some(format!(
        "config file is malformed ({} syntax {noun}); affected values are read as written, \
         stray quotes included: {}",
        problems.len(),
        locations.join("; ")
    ))
}

/// Set once a syntax warning has been printed, so it appears once per run.
/// `load_config_file` is called several times in one process (argument
/// parsing, the piped path, the REPL), and each call re-reads the same file.
static SYNTAX_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Print the #1034 warning for `content` (read from `shown`) to stderr, at
/// most once per process. Not quiet-gated, like the #1028 numeric warning: a
/// malformed file is a fault, not startup chatter.
pub(crate) fn warn_config_syntax_once(shown: &str, content: &str) {
    use std::sync::atomic::Ordering;
    if let Some(w) = config_syntax_warning(shown, &config_syntax_problems(content)) {
        if !SYNTAX_WARNED.swap(true, Ordering::SeqCst) {
            use crate::format::{RESET, YELLOW};
            eprintln!("{YELLOW}warning:{RESET} {w}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detected_shapes_fire_on_the_right_line() {
        let cases: &[(&str, Vec<(usize, &str)>)] = &[
            (
                "provider = \"anthropic\"\nmodel = \"claude-haiku-5-5\n",
                vec![(2, UNTERMINATED_QUOTE)],
            ),
            ("model = 'claude-haiku-5-5\n", vec![(1, UNTERMINATED_QUOTE)]),
            // An escaped quote is not a closing quote.
            ("model = \"abc\\\"\n", vec![(1, UNTERMINATED_QUOTE)]),
            (
                "[permissions]\nallow = [\"a\", \"b]\n",
                vec![(2, UNTERMINATED_QUOTE)],
            ),
            (
                "model = \"x\"\n\n[permissions\nallow = [\"a\"]\n",
                vec![(3, UNCLOSED_HEADER)],
            ),
            (
                "model = \"x\n[mcp_servers.fs\n",
                vec![(1, UNTERMINATED_QUOTE), (2, UNCLOSED_HEADER)],
            ),
        ];
        for (text, want) in cases {
            assert_eq!(&config_syntax_problems(text), want, "input: {text:?}");
        }
    }

    #[test]
    fn near_misses_report_nothing() {
        let clean = [
            "model = \"claude-haiku-5-5\"",
            "model = 'claude-haiku-5-5'",
            "model = \"x\" # the model I use",
            "model = \"a # not a comment\"",
            "notify_command = \"echo \\\"done\\\"\"",
            "path = 'C:\\dir\\'",
            "allow = [\"a\", \"b\"]",
            "allow = ['a', \"it's\"]",
            "env = { KEY = \"v\" }",
            "max_tokens = 4096",
            "notify_command = echo it's done",
            "prompt = \"\"\"",
            "allow = [",
            "  \"git *\",",
            "]",
            "[mcp_servers.x]",
            "[permissions] # comment",
            "# model = \"unterminated in a comment",
            "",
            "key = \"✓ multi-byte ✓\"",
        ]
        .join("\n");
        assert_eq!(config_syntax_problems(&clean), Vec::new());
    }

    #[test]
    fn composer_is_none_for_no_problems() {
        assert_eq!(config_syntax_warning(".yoyo.toml", &[]), None);
    }

    #[test]
    fn composer_renders_full_warning() {
        assert_eq!(
            config_syntax_warning(".yoyo.toml", &[(2, UNTERMINATED_QUOTE)]).as_deref(),
            Some(
                "config file is malformed (1 syntax problem); affected values are read as \
                 written, stray quotes included: .yoyo.toml:2: unterminated quote"
            )
        );
        assert_eq!(
            config_syntax_warning("a.toml", &[(1, UNTERMINATED_QUOTE), (4, UNCLOSED_HEADER)])
                .as_deref(),
            Some(
                "config file is malformed (2 syntax problems); affected values are read as \
                 written, stray quotes included: a.toml:1: unterminated quote; \
                 a.toml:4: unclosed [section] header"
            )
        );
    }

    #[test]
    fn composer_escapes_control_bytes_in_path() {
        let path = "evil\x1b[31m.toml";
        assert!(path.as_bytes().contains(&0x1b), "fixture must be hostile");
        let out = config_syntax_warning(path, &[(1, UNCLOSED_HEADER)]).unwrap();
        assert!(!out.as_bytes().contains(&0x1b), "got {out:?}");
    }
}
