//! Streamed-vs-whole parity for `MarkdownRenderer` (#1029).
//!
//! Lives beside `markdown.rs` (declared there with `#[path]`, so it is a child
//! module and can reach the renderer's private helpers) because `markdown.rs`
//! is on the module-size grandfather list.
//!
//! Why this file exists: the renderer has two ways to reach the same bytes —
//! a whole line arriving at once (`render_line`) and the same line arriving as
//! streaming deltas (the line-start buffer, the early block-prefix resolve and
//! the mid-line fast path with the #661 inline carry). Nothing compared them,
//! which is how `["- **", "apply**: x\n"]` shipped as a literal `**apply**`
//! while the whole line rendered bold. Every fixture here is fed whole, one
//! char per delta, and through the exact splits that leaked, and the outputs
//! are compared with `assert_eq!` on the full string.

use super::*;

/// Reference rendering: the whole fixture as one delta, then `flush`.
fn render_whole(input: &str) -> String {
    let mut r = MarkdownRenderer::new();
    let mut out = r.render_delta(input);
    out.push_str(&r.flush());
    out
}

/// Render `chunks` as successive deltas, then `flush`.
fn render_chunks(chunks: &[&str]) -> String {
    let mut r = MarkdownRenderer::new();
    let mut out = String::new();
    for c in chunks {
        out.push_str(&r.render_delta(c));
    }
    out.push_str(&r.flush());
    out
}

/// One delta per line (each including its `\n`).
fn render_per_line(input: &str) -> String {
    let lines: Vec<&str> = input.split_inclusive('\n').collect();
    render_chunks(&lines)
}

/// One delta per char (UTF-8 safe: splits on char boundaries).
fn render_per_char(input: &str) -> String {
    let mut buf = [0u8; 4];
    let chars: Vec<String> = input
        .chars()
        .map(|c| c.encode_utf8(&mut buf).to_string())
        .collect();
    let refs: Vec<&str> = chars.iter().map(String::as_str).collect();
    render_chunks(&refs)
}

/// Drop ANSI CSI sequences (`ESC [ ... letter`), leaving the visible text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

const HEADINGS: &[&str] = &[
    "# Top level\n",
    "## How it all fits together\n",
    "### Third level\n",
    "## The **apply** step\n",
];

/// Non-code fixtures: whole, per-line and per-char output must be byte-identical.
const PROSE: &[&str] = &[
    "# Top level\n",
    "## How it all fits together\n",
    "### Third level\n",
    "## The **apply** step\n",
    "- **apply**: x\n",
    "1. **apply** does\n",
    "Some **bold** and *it* text here\n",
    "> **ap**ply x\n",
    "> **apply** x\n",
    "a `**x**` b\n",
    "Intro line\n## The **apply** step\n- **apply**: x\nafter\n",
];

/// The exact splits from #1029, each paired with the line they spell.
const SPLITS: &[&[&str]] = &[
    &["- **", "apply**: x\n"],
    &["-", " **apply", "**: does X\n"],
    &["1", ".", " **app", "ly** does\n"],
    &[">", " **ap", "ply** x\n"],
    &["## The **ap", "ply** step\n"],
    &["## How it all ", "fits together\n"],
];

#[test]
fn fixtures_really_contain_the_markers_under_test() {
    // Anti-vacuous: a transcription slip that dropped the `#` or `**` would
    // make every "no marker leaked" assertion below pass by agreeing with itself.
    for h in HEADINGS {
        assert!(h.starts_with('#'), "heading fixture lacks `#`: {h:?}");
    }
    for f in PROSE.iter().filter(|f| !f.starts_with('#')) {
        assert!(f.contains("**"), "prose fixture lacks `**`: {f:?}");
    }
    for s in SPLITS {
        let joined = s.concat();
        assert!(
            joined.contains("**") || joined.starts_with('#'),
            "split fixture lacks its marker: {s:?}"
        );
    }
}

#[test]
fn streamed_output_matches_whole_line_output() {
    for f in PROSE {
        let whole = render_whole(f);
        assert_eq!(render_per_line(f), whole, "per-line chunking of {f:?}");
        assert_eq!(render_per_char(f), whole, "per-char chunking of {f:?}");
    }
}

#[test]
fn issue_1029_exact_splits_match_whole_line_output() {
    for s in SPLITS {
        let joined = s.concat();
        assert_eq!(render_chunks(s), render_whole(&joined), "split {s:?}");
    }
}

#[test]
fn no_markdown_marker_leaks_outside_code() {
    for f in PROSE.iter().filter(|f| !f.contains('`')) {
        for out in [render_whole(f), render_per_char(f)] {
            assert!(!out.contains("**"), "`**` leaked from {f:?}: {out:?}");
        }
    }
    for s in SPLITS {
        let out = render_chunks(s);
        assert!(!out.contains("**"), "`**` leaked from split {s:?}: {out:?}");
    }
    for h in HEADINGS {
        for out in [render_whole(h), render_per_char(h)] {
            assert!(
                !out.contains('#'),
                "heading marker leaked from {h:?}: {out:?}"
            );
        }
    }
}

#[test]
fn heading_renders_marker_free_with_inline_spans_and_style_restored() {
    // The before bytes (#1029) were `\e[1m\e[36m## The **apply** step\e[0m`.
    assert_eq!(
        render_whole("## The **apply** step\n"),
        format!("{BOLD}{CYAN}The {BOLD}apply{RESET}{BOLD}{CYAN} step{RESET}\n")
    );
    assert_eq!(
        render_whole("## How it all fits together\n"),
        format!("{BOLD}{CYAN}How it all fits together{RESET}\n")
    );
    // Optional closing sequence is dropped too; a `#` inside a word is not.
    assert_eq!(
        render_whole("## Title ##\n"),
        format!("{BOLD}{CYAN}Title{RESET}\n")
    );
    assert_eq!(render_whole("## C#\n"), format!("{BOLD}{CYAN}C#{RESET}\n"));
}

#[test]
fn non_atx_hash_lines_keep_their_pre_1029_rendering() {
    // Near-miss: `#tag` and `#######` are not ATX headings (no space after,
    // or more than six `#`). They keep the old whole-line style untouched.
    for line in ["#tag", "####### seven"] {
        assert_eq!(
            render_whole(&format!("{line}\n")),
            format!("{BOLD}{CYAN}{line}{RESET}\n"),
            "{line:?}"
        );
    }
}

#[test]
fn heading_keeps_its_marker_when_colour_is_off() {
    // With colour off every style is empty, so the `#` is the only heading
    // signal left; it is kept (raw mode). Colour on drops it.
    let r = MarkdownRenderer::new();
    let plain = r.render_heading("## The **apply** step", false);
    assert!(plain.starts_with("## The "), "plain: {plain:?}");
    assert!(
        !plain.contains("**"),
        "plain still renders emphasis: {plain:?}"
    );
    let styled = r.render_heading("## The **apply** step", true);
    assert!(!styled.contains('#'), "styled: {styled:?}");
}

#[test]
fn list_prefix_split_carries_the_opener() {
    // The before bytes (#1029) were `• **apply**: x` (literal markers).
    assert_eq!(
        render_chunks(&["- **", "apply**: x\n"]),
        format!("{CYAN}•{RESET} {BOLD}apply{RESET}: x\n")
    );
}

#[test]
fn code_fence_and_span_keep_markers_literal() {
    let fence = "```\n**x**\n## not a heading\n```\n";
    assert!(fence.contains("**x**") && fence.contains("## not"));
    let whole = render_whole(fence);
    assert_eq!(render_per_line(fence), whole, "per-line chunking of fence");
    // Per-char streaming inside a fence dims each fragment separately by
    // design (#147: token-by-token code streaming), so the bytes differ
    // there; the visible text must still be identical and literal.
    let visible = strip_ansi(&render_per_char(fence));
    assert_eq!(visible, strip_ansi(&whole));
    assert!(visible.contains("**x**") && visible.contains("## not a heading"));

    let span = "a `**x**` b\n";
    assert_eq!(render_whole(span), format!("a {CYAN}**x**{RESET} b\n"));
}

// --- Basic rendering tests, moved verbatim from markdown.rs (size budget) ---

#[test]
fn test_md_code_block_detection() {
    let input = "before\n```\ncode line\n```\nafter\n";
    let out = render_whole(input);
    // "code line" should be wrapped in DIM
    assert!(out.contains(&format!("{DIM}code line{RESET}")));
    // "before" and "after" should NOT be dim
    assert!(out.contains("before"));
    assert!(out.contains("after"));
}

#[test]
fn test_md_code_block_with_language() {
    let input = "```rust\nlet x = 1;\n```\n";
    let mut r = MarkdownRenderer::new();
    let out = r.render_delta(input);
    let flushed = r.flush();
    let full = format!("{out}{flushed}");
    // Language should be captured and fence dimmed
    assert!(full.contains(&format!("{DIM}```rust{RESET}")));
    // "let" should be keyword-highlighted, not just DIM
    assert!(full.contains(&format!("{BOLD_CYAN}let{RESET}")));
    // Number should be yellow
    assert!(full.contains(&format!("{YELLOW}1{RESET}")));
}

#[test]
fn test_md_inline_code() {
    let out = render_whole("use `Option<T>` here\n");
    assert!(out.contains(&format!("{CYAN}Option<T>{RESET}")));
}

#[test]
fn test_md_bold_text() {
    let out = render_whole("this is **important** stuff\n");
    assert!(out.contains(&format!("{BOLD}important{RESET}")));
}

#[test]
fn test_md_header_rendering() {
    let out = render_whole("# Hello World\n");
    // expected value changed by #1029: this pinned the marker leak
    assert!(out.contains(&format!("{BOLD}{CYAN}Hello World{RESET}")));
}

#[test]
fn test_md_header_h2() {
    let out = render_whole("## Section Two\n");
    // expected value changed by #1029: this pinned the marker leak
    assert!(out.contains(&format!("{BOLD}{CYAN}Section Two{RESET}")));
}

#[test]
fn test_md_partial_delta_fence() {
    // Fence marker split across multiple deltas
    let mut r = MarkdownRenderer::new();
    let out1 = r.render_delta("``");
    // Nothing emitted yet — still buffered (no newline)
    assert_eq!(out1, "");
    let out2 = r.render_delta("`\n");
    // Now the fence line is complete
    assert!(out2.contains(&format!("{DIM}```{RESET}")));
    let out3 = r.render_delta("code here\n");
    assert!(out3.contains(&format!("{DIM}code here{RESET}")));
    let out4 = r.render_delta("```\n");
    assert!(out4.contains(&format!("{DIM}```{RESET}")));
    // After closing, normal text again
    let out5 = r.render_delta("normal\n");
    assert!(out5.contains("normal"));
    assert!(!out5.contains(&format!("{DIM}")));
}
