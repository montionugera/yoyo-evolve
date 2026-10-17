//! #1002 part 1: project instruction files (CLAUDE.md, AGENTS.md, …) honour
//! `--deny-dir` / `--allow-dir` at their **resolved** target, through the same
//! `DirectoryRestrictions::check_path` the file tools use since #996.
//!
//! Every fixture lives in a tempdir and is passed to the dir-taking seam
//! `load_project_context_restricted`; nothing here reads or writes the repo cwd.
#![cfg(unix)]

use crate::config::DirectoryRestrictions;
use crate::context::{
    instruction_file_refused_warning, list_project_context_files_fenced,
    load_project_context_fenced, load_project_context_from, load_project_context_restricted,
};
use std::os::unix::fs::symlink;

/// `secret/notes.md` + `secret/agents.md` holding tokens, and a `proj/` whose
/// CLAUDE.md and AGENTS.md are symlinks into `secret/`.
fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::TempDir::new().unwrap();
    let base = root.path().canonicalize().unwrap();
    let secret = base.join("secret");
    let proj = base.join("proj");
    std::fs::create_dir_all(&secret).unwrap();
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(secret.join("notes.md"), "SECRET_TOKEN_ZZ9\n").unwrap();
    std::fs::write(secret.join("agents.md"), "AGENTS_SECRET_KK3\n").unwrap();
    symlink("../secret/notes.md", proj.join("CLAUDE.md")).unwrap();
    symlink("../secret/agents.md", proj.join("AGENTS.md")).unwrap();
    (root, secret, proj)
}

fn deny(dir: &std::path::Path) -> DirectoryRestrictions {
    DirectoryRestrictions {
        allow: vec![],
        deny: vec![dir.display().to_string()],
    }
}

fn allow(dirs: &[&std::path::Path]) -> DirectoryRestrictions {
    DirectoryRestrictions {
        allow: dirs.iter().map(|d| d.display().to_string()).collect(),
        deny: vec![],
    }
}

#[test]
fn instruction_link_into_denied_dir_is_not_loaded() {
    let (_root, secret, proj) = fixture();
    // Anti-vacuous: the unrestricted loader really does read both tokens, so
    // their absence below is the fence, not a broken fixture.
    let open = load_project_context_from(&proj).expect("fixture must load");
    assert!(open.contains("SECRET_TOKEN_ZZ9") && open.contains("AGENTS_SECRET_KK3"));

    let (ctx, refused) = load_project_context_restricted(&proj, &deny(&secret));
    let ctx = ctx.unwrap_or_default();
    assert!(!ctx.contains("SECRET_TOKEN_ZZ9"), "denied CLAUDE.md leaked");
    assert!(
        !ctx.contains("AGENTS_SECRET_KK3"),
        "denied AGENTS.md leaked"
    );
    let names: Vec<&str> = refused.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, vec!["CLAUDE.md", "AGENTS.md"]);
}

#[test]
fn instruction_link_outside_allow_list_is_not_loaded() {
    let (_root, _secret, proj) = fixture();
    let (ctx, refused) = load_project_context_restricted(&proj, &allow(&[&proj]));
    let ctx = ctx.unwrap_or_default();
    assert!(!ctx.contains("SECRET_TOKEN_ZZ9"));
    assert!(!ctx.contains("AGENTS_SECRET_KK3"));
    assert_eq!(refused.len(), 2);
}

#[test]
fn instruction_link_resolving_inside_allowed_set_still_loads() {
    // Near-miss: same links, but the target is inside the allowed set. The
    // check is on the resolved target, so it must pass — the same verdict the
    // file tools give a link entry whose target is allowed (#998 consistency).
    let (_root, secret, proj) = fixture();
    let (ctx, refused) = load_project_context_restricted(&proj, &allow(&[&proj, &secret]));
    let ctx = ctx.expect("allowed links must load");
    assert!(ctx.contains("SECRET_TOKEN_ZZ9") && ctx.contains("AGENTS_SECRET_KK3"));
    assert!(refused.is_empty());
}

#[test]
fn regular_file_and_no_restrictions_are_byte_identical() {
    // The whole regression surface: every user without --deny-dir/--allow-dir,
    // and every plain instruction file inside an allowed dir.
    let root = tempfile::TempDir::new().unwrap();
    let proj = root.path().canonicalize().unwrap();
    std::fs::write(proj.join("CLAUDE.md"), "PLAIN_RULES_QQ1\n").unwrap();
    let before = load_project_context_from(&proj);
    assert!(before.as_deref().unwrap_or("").contains("PLAIN_RULES_QQ1"));

    let (none, refused) = load_project_context_restricted(&proj, &DirectoryRestrictions::default());
    assert_eq!(none, before);
    assert!(refused.is_empty());

    let (inside, refused) = load_project_context_restricted(&proj, &allow(&[&proj]));
    assert_eq!(inside, before);
    assert!(refused.is_empty());
}

#[test]
fn refused_warning_names_file_sanitizes_and_is_plain_safe() {
    let reason = "Access denied: '/x/\u{1b}[31mCLAUDE.md' is under restricted directory '/x'";
    assert!(reason.as_bytes().contains(&0x1b), "anti-vacuous fixture");
    for plain in [false, true] {
        let w = instruction_file_refused_warning("CLAUDE.md", reason, plain);
        assert!(w.contains("CLAUDE.md"));
        assert!(w.contains("not loaded"));
        assert!(
            !w.as_bytes().contains(&0x1b),
            "control byte reached the terminal"
        );
        if plain {
            assert!(w.is_ascii(), "plain output must be glyph-free: {w}");
        }
    }
}

// ---- #1002 part 3: `.yoyo/memory.json` behind the same fence ----

const MEM_TOKEN: &str = "MEMTOK_1002_ZQ";
const MEM_JSON: &str = r#"{"entries":[{"note":"MEMTOK_1002_ZQ","timestamp":"2026-01-01 00:00","category":"general"}]}"#;
/// The exact section `format_memories_for_prompt` renders for `MEM_JSON`,
/// typed here as the before-state the near-miss must keep byte-identical.
const MEM_SECTION: &str = "## Project Memories\n\n- MEMTOK_1002_ZQ (2026-01-01 00:00)";

/// `secret/mem.json` holding the token, and `proj/.yoyo/memory.json` a symlink to it.
fn memory_fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::TempDir::new().unwrap();
    let base = root.path().canonicalize().unwrap();
    let secret = base.join("secret");
    let proj = base.join("proj");
    std::fs::create_dir_all(&secret).unwrap();
    std::fs::create_dir_all(proj.join(".yoyo")).unwrap();
    std::fs::write(secret.join("mem.json"), MEM_JSON).unwrap();
    symlink("../../secret/mem.json", proj.join(".yoyo/memory.json")).unwrap();
    // Anti-vacuous: the fixture file really carries the token, and the
    // unrestricted loader really reads it through the link.
    let raw = std::fs::read_to_string(secret.join("mem.json")).unwrap();
    assert!(raw.contains(MEM_TOKEN), "fixture must contain the token");
    let open = load_project_context_from(&proj).unwrap_or_default();
    assert!(open.contains(MEM_TOKEN), "unfenced load must read the link");
    (root, secret, proj)
}

#[test]
fn memory_link_into_denied_dir_is_not_loaded() {
    let (_root, secret, proj) = memory_fixture();
    let (ctx, refused) = load_project_context_restricted(&proj, &deny(&secret));
    let ctx = ctx.unwrap_or_default();
    assert!(
        !ctx.contains(MEM_TOKEN),
        "denied memory reached the prompt: {ctx}"
    );
    assert!(!ctx.contains("## Project Memories"));
    let names: Vec<&str> = refused.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, vec![".yoyo/memory.json"]);
}

#[test]
fn memory_link_outside_allow_list_is_not_loaded() {
    let (_root, _secret, proj) = memory_fixture();
    let (ctx, refused) = load_project_context_restricted(&proj, &allow(&[&proj]));
    assert!(!ctx.unwrap_or_default().contains(MEM_TOKEN));
    assert_eq!(refused.len(), 1);
}

#[test]
fn memory_link_with_fence_not_covering_target_still_loads() {
    // Near-miss: a deny list that names some other directory does not cover
    // the link's target, so the memory must load.
    let (root, _secret, proj) = memory_fixture();
    let other = root.path().canonicalize().unwrap().join("other");
    std::fs::create_dir_all(&other).unwrap();
    let (ctx, refused) = load_project_context_restricted(&proj, &deny(&other));
    assert!(ctx.unwrap_or_default().contains(MEM_TOKEN));
    assert!(refused.is_empty());
}

#[test]
fn plain_memory_file_without_fence_is_byte_identical() {
    // The regression surface: an ordinary memory.json, no fence. The section
    // must be exactly what it was before #1002 part 3.
    let root = tempfile::TempDir::new().unwrap();
    let proj = root.path().canonicalize().unwrap();
    std::fs::create_dir_all(proj.join(".yoyo")).unwrap();
    std::fs::write(proj.join(".yoyo/memory.json"), MEM_JSON).unwrap();
    let before = load_project_context_from(&proj).expect("memory must load");
    let start = before.find("## Project Memories").expect("memory section");
    assert_eq!(&before[start..], MEM_SECTION);

    let (fenced, refused) =
        load_project_context_restricted(&proj, &DirectoryRestrictions::default());
    assert_eq!(fenced.as_deref(), Some(before.as_str()));
    assert!(refused.is_empty());
    let (inside, refused) = load_project_context_restricted(&proj, &allow(&[&proj]));
    assert_eq!(inside.as_deref(), Some(before.as_str()));
    assert!(refused.is_empty());
}

#[test]
fn missing_memory_file_is_silent_under_a_fence() {
    let root = tempfile::TempDir::new().unwrap();
    let proj = root.path().canonicalize().unwrap();
    let (_, refused) = load_project_context_restricted(&proj, &deny(&proj.join("nope")));
    assert!(refused.is_empty());
    let (_, refused) = load_project_context_restricted(&proj, &allow(&[&proj.join("x")]));
    assert!(refused.is_empty(), "absent memory.json must not warn");
}

#[test]
fn memory_refused_warning_names_memory_not_instructions() {
    for plain in [false, true] {
        let w = instruction_file_refused_warning(".yoyo/memory.json", "Access denied", plain);
        assert!(w.contains(".yoyo/memory.json") && w.contains("not loaded"));
        assert!(w.contains("memory file"), "{w}");
        assert!(!w.contains("instruction file"), "{w}");
    }
}

// ── #1042: the /cd-applied [directories] lists reach the loader ────────────
// `/spawn` reloads instruction files after `/cd`; these drive the loader over
// local `/cd` handles, never the process-wide lists.

fn handle(entries: &[&std::path::Path]) -> crate::cd_deny::DenyAdditions {
    std::sync::Arc::new(std::sync::Mutex::new(
        entries.iter().map(|d| d.display().to_string()).collect(),
    ))
}

/// `proj/CLAUDE.md` holding a canary, plus a sibling `other/` dir.
fn cd_fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::TempDir::new().unwrap();
    let base = root.path().canonicalize().unwrap();
    let proj = base.join("proj");
    let other = base.join("other");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(proj.join("CLAUDE.md"), "CD_CANARY_JX4\n").unwrap();
    (root, proj, other)
}

#[test]
fn cd_deny_list_keeps_instruction_file_out_of_spawn_context() {
    let (_root, proj, _other) = cd_fixture();
    // Anti-vacuous: the fixture really carries the canary and the open loader reads it.
    let raw = std::fs::read_to_string(proj.join("CLAUDE.md")).unwrap();
    assert!(raw.contains("CD_CANARY_JX4"));
    let open = load_project_context_from(&proj).expect("fixture must load");
    assert!(open.contains("CD_CANARY_JX4"));

    let (ctx, refused) = load_project_context_fenced(
        &proj,
        &DirectoryRestrictions::default(),
        &handle(&[&proj]),
        &handle(&[]),
    );
    assert!(
        !ctx.unwrap_or_default().contains("CD_CANARY_JX4"),
        "/cd-denied CLAUDE.md leaked into the loaded context"
    );
    let names: Vec<&str> = refused.iter().map(|(n, _)| *n).collect();
    assert!(
        names.contains(&"CLAUDE.md"),
        "refusal not reported: {names:?}"
    );
}

#[test]
fn cd_allow_list_not_covering_file_keeps_it_out() {
    let (_root, proj, other) = cd_fixture();
    let (ctx, _) = load_project_context_fenced(
        &proj,
        &DirectoryRestrictions::default(),
        &handle(&[]),
        &handle(&[&other]),
    );
    assert!(!ctx.unwrap_or_default().contains("CD_CANARY_JX4"));
}

#[test]
fn empty_cd_lists_are_byte_identical_to_before() {
    let (_root, proj, _other) = cd_fixture();
    let before = load_project_context_from(&proj);
    assert!(before.as_deref().unwrap_or("").contains("CD_CANARY_JX4"));
    let (ctx, refused) = load_project_context_fenced(
        &proj,
        &DirectoryRestrictions::default(),
        &handle(&[]),
        &handle(&[]),
    );
    assert_eq!(ctx, before);
    assert!(refused.is_empty());
}

#[test]
fn cd_deny_of_a_different_dir_leaves_file_loaded() {
    let (_root, proj, other) = cd_fixture();
    let before = load_project_context_from(&proj);
    let (ctx, refused) = load_project_context_fenced(
        &proj,
        &DirectoryRestrictions::default(),
        &handle(&[&other]),
        &handle(&[]),
    );
    assert_eq!(ctx, before);
    assert!(refused.is_empty());
}

// #1043: `/context` and `/doctor` list instruction files through the same
// fence the loader uses, so the display cannot list a file the loader refuses.

#[test]
fn cd_denied_file_is_refused_by_both_loader_and_lister() {
    let (_root, proj, _other) = cd_fixture();
    let (deny, allow) = (handle(&[&proj]), handle(&[]));
    let none = DirectoryRestrictions::default();
    let (ctx, loader_refused) = load_project_context_fenced(&proj, &none, &deny, &allow);
    assert!(!ctx.unwrap_or_default().contains("CD_CANARY_JX4"));
    assert!(loader_refused.iter().any(|(n, _)| *n == "CLAUDE.md"));

    let (listed, refused) = list_project_context_files_fenced(&proj, &none, &deny, &allow);
    assert!(
        !listed.iter().any(|(n, _)| *n == "CLAUDE.md"),
        "lister shows a file the loader refuses: {listed:?}"
    );
    let names: Vec<&str> = refused.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, vec!["CLAUDE.md"], "lister and loader disagree");
    assert!(!refused[0].1.is_empty(), "refusal carries no reason");
}

#[test]
fn file_outside_the_cd_denied_dir_is_still_listed_with_its_lines() {
    let (_root, proj, other) = cd_fixture();
    std::fs::write(proj.join("AGENTS.md"), "one\ntwo\nthree\n").unwrap();
    let (listed, refused) = list_project_context_files_fenced(
        &proj,
        &DirectoryRestrictions::default(),
        &handle(&[&other]),
        &handle(&[]),
    );
    assert_eq!(listed, vec![("CLAUDE.md", 1), ("AGENTS.md", 3)]);
    assert!(refused.is_empty());
}

#[test]
fn startup_deny_refuses_in_the_lister_too() {
    let (_root, proj, _other) = cd_fixture();
    let (listed, refused) =
        list_project_context_files_fenced(&proj, &deny(&proj), &handle(&[]), &handle(&[]));
    assert!(listed.is_empty(), "{listed:?}");
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].0, "CLAUDE.md");
}

#[test]
fn no_fence_lists_exactly_what_the_unfenced_read_finds() {
    let (_root, proj, _other) = cd_fixture();
    std::fs::write(proj.join("AGENTS.md"), "a\nb\n").unwrap();
    std::fs::write(proj.join(".cursorrules"), "   \n").unwrap(); // blank: not listed
                                                                 // The pre-#1043 loop, verbatim but over `proj`: a bare read of every name.
    let mut before = Vec::new();
    for name in crate::context::PROJECT_CONTEXT_FILES {
        if let Ok(content) = std::fs::read_to_string(proj.join(name)) {
            let content = content.trim();
            if !content.is_empty() {
                before.push((*name, content.lines().count()));
            }
        }
    }
    assert_eq!(
        before.len(),
        2,
        "anti-vacuous: fixture has two non-blank files"
    );
    let (listed, refused) = list_project_context_files_fenced(
        &proj,
        &DirectoryRestrictions::default(),
        &handle(&[]),
        &handle(&[]),
    );
    assert_eq!(listed, before);
    assert!(refused.is_empty());
}

#[test]
fn refused_context_row_names_file_sanitizes_reason_and_has_no_glyph() {
    let row = crate::commands_project::refused_context_file_line("CLAUDE.md", "denied: /x\x1b[31m");
    assert!(
        !row.as_bytes().contains(&0x1b),
        "raw ESC reached the row: {row:?}"
    );
    assert!(row.starts_with("CLAUDE.md (refused by the directory fence: denied: /x"));
    assert!(row.is_ascii(), "row carries a glyph: {row:?}");
    let plain = crate::commands_project::refused_context_file_line("AGENTS.md", "outside allow");
    assert_eq!(
        plain,
        "AGENTS.md (refused by the directory fence: outside allow)"
    );
}
