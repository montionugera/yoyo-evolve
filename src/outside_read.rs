//! Opt-in ask-once fence for `read_file`, `search` and `list_files` outside
//! the working directory (part of #879).
//!
//! **Off by default, and off means byte-identical.** The fence is active only
//! when the environment variable `YOYO_ASK_OUTSIDE_READS` is exactly `1`, read
//! once per process. Unset, or any other value, and [`check`] returns `None`
//! for every call, so the file tool behaves exactly as before. The default is
//! deliberately unchanged: peers disagree (Claude Code asks before reading above
//! the launch folder, Codex's read-only sandbox reads anywhere), and a default
//! that asks would break every piped/CI user with no terminal to answer (#448).
//!
//! **What "outside" means.** The target is resolved with `fs::canonicalize`
//! (symlinks followed, `..` removed; a relative path is joined to the current
//! directory first), the same resolution the directory fence uses for #996, and
//! compared against the canonicalized current directory. A symlink inside the
//! working directory that points outside it is outside. A path that cannot be
//! resolved (it does not exist) is not asked about: `read_file` reports its
//! own error, and there is nothing to read.
//!
//! **Which tools, and which argument.** `read_file`'s `path`, and the ROOT
//! (`path`) of `search` and `list_files`. A missing root means the tool's
//! default, the current directory, which is inside, so nothing is asked (the
//! wrapper only calls [`check`] when `path` is present).
//!
//! **Ask once per directory, and the remembered key.** A yes is remembered for
//! the rest of the session as an [`Approval`]:
//! - a FILE target (any `read_file`, or a `search`/`list_files` root that is a
//!   file) is remembered as `Files(parent)`, the canonical parent directory,
//!   exactly, so three files in `/etc` ask once, and `search` on one of them
//!   rides the same yes;
//! - a DIRECTORY root of `search`/`list_files` is remembered as `Tree(dir)`,
//!   the canonical directory itself, exactly. Asking about `/a/b` does not
//!   cover a later root at `/a/b/c` (that asks again), the same exact-directory
//!   rule `read_file` uses.
//!
//! The two kinds never cover each other, because both tools walk recursively:
//! a yes to reading `~/.bashrc` (`Files(~)`) must not quietly grant a search
//! rooted at `~`, which would reach `~/.ssh`. Neither kind covers
//! subdirectories. A no refuses this call only and is not remembered, so the
//! next call there asks again.
//!
//! **No terminal, no prompt.** When stdin is not a terminal (`-p` in a pipe,
//! CI) the read is refused, and the refusal names the path and the ways to
//! allow it. The model receives it as the tool result. It carries the
//! `REFUSAL_STEM_MODE_ACTIVE` stem so the recovery-hint wrapper treats it as a
//! deliberate refusal rather than a flaky tool (#710).
//!
//! **`--allow-dir` is consent.** When the startup `--allow-dir` list is set, a
//! path has already passed it by the time this runs, so the user named that
//! directory and nothing is asked.
//!
//! **Not a sandbox.** This fences the three read tools' path argument only.
//! `bash` can still `cat` the same file, writes (`write_file`, `edit_file`,
//! `rename_symbol`) are a different fence, and a search rooted inside the cwd
//! still follows whatever its own walker follows. `--restricted` and
//! `--allow-dir` remain the hard fences.
//!
//! **Why not `confirm_file_operation`.** That helper's `a`/always answer flips a
//! session-wide auto-approve flag and offers to write a `[permissions] allow`
//! pattern to `.yoyo.toml`, neither of which means anything for a read fence,
//! so the prompt here is a plain y/n in the same visual style.

use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};

use crate::format::{GREEN, RED, RESET, YELLOW};

/// The opt-in switch.
pub(crate) const ENV_VAR: &str = "YOYO_ASK_OUTSIDE_READS";

/// The tools whose `path` argument this fence reads.
const FENCED_TOOLS: &[&str] = &["read_file", "search", "list_files"];

static ENABLED: OnceLock<bool> = OnceLock::new();

/// What a yes covers for the rest of the session (see the module docs).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Approval {
    /// Files directly in this canonical directory (a file target's parent).
    Files(PathBuf),
    /// A recursive `search`/`list_files` rooted at exactly this canonical directory.
    Tree(PathBuf),
}

impl Approval {
    fn dir(&self) -> &Path {
        match self {
            Approval::Files(d) | Approval::Tree(d) => d,
        }
    }
}

/// Approvals granted this session.
static APPROVED: LazyLock<Mutex<HashSet<Approval>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Only the exact value `1` turns the fence on.
fn enabled_from(value: Option<&str>) -> bool {
    value == Some("1")
}

fn is_enabled() -> bool {
    *ENABLED.get_or_init(|| enabled_from(std::env::var(ENV_VAR).ok().as_deref()))
}

/// The canonical target when `path` resolves to somewhere outside `cwd`;
/// `None` when it is inside, or when it cannot be resolved at all.
fn resolve_outside(path: &str, cwd: &Path) -> Option<PathBuf> {
    let p = Path::new(path);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    let target = std::fs::canonicalize(joined).ok()?;
    let root = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    if target.starts_with(&root) {
        None
    } else {
        Some(target)
    }
}

/// What a yes for `tool` on the canonical `target` is remembered as.
/// `read_file` is always a file read; a `search`/`list_files` root is a
/// `Tree` when it is a directory and a file read otherwise.
fn approval_for(tool: &str, target: &Path) -> Approval {
    if tool != "read_file" && target.is_dir() {
        return Approval::Tree(target.to_path_buf());
    }
    Approval::Files(
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| target.to_path_buf()),
    )
}

/// The refusal a non-interactive run gets.
fn no_tty_refusal(tool: &str, target: &Path, cwd: &Path, approval: &Approval) -> String {
    let (holder, named) = match approval {
        Approval::Files(_) => ("the file", "its directory"),
        Approval::Tree(_) => ("that directory", "that directory"),
    };
    format!(
        "{ENV_VAR}=1{stem}{tool} refused '{}': it is outside the working directory '{}', \
         and there is no terminal to ask on. To allow it, unset {ENV_VAR}, run yoyo from a \
         directory that contains {holder}, or name {named} with --allow-dir.",
        target.display(),
        cwd.display(),
        stem = crate::tool_wrappers::REFUSAL_STEM_MODE_ACTIVE,
    )
}

/// The refusal an interactive "no" gets.
fn declined_refusal(tool: &str, target: &Path) -> String {
    format!(
        "{}{tool} on '{}' (outside the working directory)",
        crate::tool_wrappers::REFUSAL_STEM_PATH_DENIED,
        target.display()
    )
}

/// The whole decision, with every input injected. `None` = proceed.
#[allow(clippy::too_many_arguments)]
fn decide(
    enabled: bool,
    tool: &str,
    path: &str,
    explicit_allow: bool,
    cwd: &Path,
    store: &Mutex<HashSet<Approval>>,
    is_tty: bool,
    ask: &mut dyn FnMut(&str, &Path, &Approval) -> bool,
) -> Option<String> {
    if !enabled || !FENCED_TOOLS.contains(&tool) || explicit_allow {
        return None;
    }
    let target = resolve_outside(path, cwd)?;
    let approval = approval_for(tool, &target);
    // Held across the prompt so two parallel calls cannot both ask.
    let mut approved = crate::sync_util::lock_or_recover(store);
    if approved.contains(&approval) {
        return None;
    }
    if !is_tty {
        return Some(no_tty_refusal(tool, &target, cwd, &approval));
    }
    if ask(tool, &target, &approval) {
        approved.insert(approval);
        None
    } else {
        Some(declined_refusal(tool, &target))
    }
}

/// Ask on stderr, read one line from stdin.
fn ask_on_terminal(tool: &str, target: &Path, approval: &Approval) -> bool {
    let target = crate::cli::sanitize_for_display(&target.display().to_string());
    let dir = crate::cli::sanitize_for_display(&approval.dir().display().to_string());
    let scope = match approval {
        Approval::Files(_) => format!("reads in {dir}"),
        Approval::Tree(_) => format!("{tool} over all of {dir} (recursive)"),
    };
    eprint!(
        "{YELLOW}  ⚠ {tool} wants '{target}', outside the working directory. Allow {scope} this session? {RESET}({GREEN}y{RESET}/{RED}n{RESET}) "
    );
    io::stderr().flush().ok();
    let mut response = String::new();
    use std::io::BufRead;
    if io::stdin().lock().read_line(&mut response).is_err() {
        return false;
    }
    matches!(response.trim().to_lowercase().as_str(), "y" | "yes")
}

/// The wrapper's entry point: `None` = proceed, `Some(refusal)` = refuse.
pub(crate) fn check(
    tool: &str,
    path: &str,
    restrictions: &crate::cli::DirectoryRestrictions,
) -> Option<String> {
    if !is_enabled() {
        return None;
    }
    let cwd = std::env::current_dir().ok()?;
    decide(
        true,
        tool,
        path,
        !restrictions.allow.is_empty(),
        &cwd,
        &APPROVED,
        io::stdin().is_terminal(),
        &mut ask_on_terminal,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// A temp tree: `<root>/work` (the cwd), `<root>/a/{x,y}`, `<root>/b/z`.
    fn tree() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        let work = root.join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("inside.txt"), "in").unwrap();
        for d in ["a", "b"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        for f in ["a/x", "a/y", "b/z", "outside.txt"] {
            std::fs::write(root.join(f), "out").unwrap();
        }
        (tmp, root, work)
    }

    fn store() -> Mutex<HashSet<Approval>> {
        Mutex::new(HashSet::new())
    }

    #[test]
    fn enabled_only_by_exact_one() {
        assert!(enabled_from(Some("1")));
        for v in [None, Some(""), Some("0"), Some("true"), Some(" 1")] {
            assert!(!enabled_from(v), "{v:?} must not enable the fence");
        }
    }

    #[test]
    fn predicate_inside_outside_symlink_and_missing() {
        let (_tmp, root, work) = tree();
        assert_eq!(resolve_outside("inside.txt", &work), None);
        assert_eq!(
            resolve_outside(work.join("inside.txt").to_str().unwrap(), &work),
            None
        );
        assert_eq!(
            resolve_outside("../outside.txt", &work),
            Some(root.join("outside.txt"))
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("outside.txt"), work.join("link")).unwrap();
            assert_eq!(
                resolve_outside("link", &work),
                Some(root.join("outside.txt")),
                "a symlink inside the cwd that points outside is outside"
            );
        }
        assert_eq!(resolve_outside("../does-not-exist", &work), None);
    }

    #[test]
    fn nonexistent_path_is_not_asked() {
        let (_tmp, _root, work) = tree();
        let asked = Cell::new(0);
        let mut ask = |_: &str, _: &Path, _: &Approval| {
            asked.set(asked.get() + 1);
            true
        };
        let s = store();
        let out = decide(
            true,
            "read_file",
            "../nope",
            false,
            &work,
            &s,
            true,
            &mut ask,
        );
        assert_eq!(out, None);
        assert_eq!(asked.get(), 0);
    }

    #[test]
    fn ask_once_per_parent_directory() {
        let (_tmp, root, work) = tree();
        let s = store();
        let asked = std::cell::RefCell::new(Vec::new());
        let mut ask = |_: &str, _: &Path, a: &Approval| {
            asked.borrow_mut().push(a.dir().to_path_buf());
            true
        };
        for p in ["../a/x", "../a/y", "../b/z"] {
            assert_eq!(
                decide(true, "read_file", p, false, &work, &s, true, &mut ask),
                None
            );
        }
        assert_eq!(*asked.borrow(), vec![root.join("a"), root.join("b")]);
    }

    #[test]
    fn a_no_refuses_and_is_not_remembered() {
        let (_tmp, root, work) = tree();
        let s = store();
        let asked = Cell::new(0);
        let mut no = |_: &str, _: &Path, _: &Approval| {
            asked.set(asked.get() + 1);
            false
        };
        let out = decide(true, "read_file", "../a/x", false, &work, &s, true, &mut no);
        assert_eq!(
            out,
            Some(format!(
                "User denied read_file on '{}' (outside the working directory)",
                root.join("a/x").display()
            ))
        );
        decide(true, "read_file", "../a/x", false, &work, &s, true, &mut no);
        assert_eq!(asked.get(), 2, "a no must ask again next time");
        assert!(crate::tool_wrappers::is_deterministic_refusal(
            &out.unwrap()
        ));
    }

    #[test]
    fn non_interactive_refusal_full_text_and_no_ansi() {
        let cwd = Path::new("/work/proj");
        let msg = no_tty_refusal(
            "read_file",
            Path::new("/etc/hosts"),
            cwd,
            &Approval::Files(PathBuf::from("/etc")),
        );
        assert_eq!(
            msg,
            "YOYO_ASK_OUTSIDE_READS=1 is active — read_file refused '/etc/hosts': it is outside \
             the working directory '/work/proj', and there is no terminal to ask on. To allow it, \
             unset YOYO_ASK_OUTSIDE_READS, run yoyo from a directory that contains the file, or \
             name its directory with --allow-dir."
        );
        assert!(!msg.as_bytes().contains(&0x1b));
        assert!(crate::tool_wrappers::is_deterministic_refusal(&msg));
    }

    #[test]
    fn non_interactive_refuses_without_asking() {
        let (_tmp, root, work) = tree();
        let s = store();
        let mut ask =
            |_: &str, _: &Path, _: &Approval| -> bool { panic!("must not prompt without a tty") };
        let out = decide(
            true,
            "read_file",
            "../outside.txt",
            false,
            &work,
            &s,
            false,
            &mut ask,
        );
        assert_eq!(
            out,
            Some(no_tty_refusal(
                "read_file",
                &root.join("outside.txt"),
                &work,
                &Approval::Files(root.clone())
            ))
        );
        // An inside read is untouched even with no tty.
        assert_eq!(
            decide(
                true,
                "read_file",
                "inside.txt",
                false,
                &work,
                &s,
                false,
                &mut ask
            ),
            None
        );
    }

    #[test]
    fn disabled_other_tools_and_allow_dir_are_untouched() {
        let (_tmp, _root, work) = tree();
        let s = store();
        let mut ask = |_: &str, _: &Path, _: &Approval| -> bool { panic!("must not prompt") };
        // Disabled: the regression surface, every user who has not opted in.
        assert_eq!(
            decide(
                false,
                "read_file",
                "../outside.txt",
                false,
                &work,
                &s,
                false,
                &mut ask
            ),
            None
        );
        // Disabled is a passthrough for the two root-taking tools too.
        for tool in ["search", "list_files"] {
            assert_eq!(
                decide(false, tool, "../a", false, &work, &s, false, &mut ask),
                None
            );
        }
        // Writes and anything unnamed are a different fence.
        for tool in ["write_file", "edit_file", "rename_symbol", "bash"] {
            assert_eq!(
                decide(
                    true,
                    tool,
                    "../outside.txt",
                    false,
                    &work,
                    &s,
                    false,
                    &mut ask
                ),
                None
            );
        }
        // --allow-dir already passed: consent given.
        assert_eq!(
            decide(
                true,
                "read_file",
                "../outside.txt",
                true,
                &work,
                &s,
                false,
                &mut ask
            ),
            None
        );
    }

    /// Records every prompt as (tool, approval) and answers `yes`.
    fn recorder(
        log: &std::cell::RefCell<Vec<(String, Approval)>>,
        yes: bool,
    ) -> impl FnMut(&str, &Path, &Approval) -> bool + '_ {
        move |tool: &str, _: &Path, a: &Approval| {
            log.borrow_mut().push((tool.to_string(), a.clone()));
            yes
        }
    }

    #[test]
    fn search_and_list_files_outside_roots_ask_or_refuse() {
        let (_tmp, root, work) = tree();
        for tool in ["search", "list_files"] {
            // No tty: refused, naming the tool, the canonical root and --allow-dir.
            let s = store();
            let mut never = |_: &str, _: &Path, _: &Approval| -> bool { panic!("no tty") };
            let out = decide(true, tool, "../a", false, &work, &s, false, &mut never);
            assert_eq!(
                out,
                Some(format!(
                    "YOYO_ASK_OUTSIDE_READS=1 is active — {tool} refused '{}': it is outside \
                     the working directory '{}', and there is no terminal to ask on. To allow \
                     it, unset YOYO_ASK_OUTSIDE_READS, run yoyo from a directory that contains \
                     that directory, or name that directory with --allow-dir.",
                    root.join("a").display(),
                    work.display()
                ))
            );
            assert!(crate::tool_wrappers::is_deterministic_refusal(
                out.as_deref().unwrap()
            ));
            // Tty: asked once, keyed on the directory itself, as a Tree.
            let log = std::cell::RefCell::new(Vec::new());
            let mut ask = recorder(&log, true);
            assert_eq!(
                decide(true, tool, "../a", false, &work, &s, true, &mut ask),
                None
            );
            assert_eq!(
                decide(true, tool, "../a", false, &work, &s, true, &mut ask),
                None
            );
            assert_eq!(
                *log.borrow(),
                vec![(tool.to_string(), Approval::Tree(root.join("a")))]
            );
            // A no names the tool and is not remembered.
            let s2 = store();
            let mut no = |_: &str, _: &Path, _: &Approval| false;
            assert_eq!(
                decide(true, tool, "../b", false, &work, &s2, true, &mut no),
                Some(format!(
                    "User denied {tool} on '{}' (outside the working directory)",
                    root.join("b").display()
                ))
            );
        }
    }

    #[test]
    fn inside_roots_pass_untouched() {
        let (_tmp, _root, work) = tree();
        std::fs::create_dir_all(work.join("sub")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(work.join("sub"), work.join("inlink")).unwrap();
        let s = store();
        let mut never = |_: &str, _: &Path, _: &Approval| -> bool { panic!("inside: no prompt") };
        // `.` is also what a missing root means (the tools default to the cwd).
        let mut roots = vec![".", "sub", "inside.txt"];
        if cfg!(unix) {
            roots.push("inlink");
        }
        for tool in ["search", "list_files"] {
            for r in &roots {
                assert_eq!(
                    decide(true, tool, r, false, &work, &s, false, &mut never),
                    None,
                    "{tool} rooted at {r} is inside"
                );
            }
            // A nonexistent root is not asked about: the tool reports it.
            assert_eq!(
                decide(true, tool, "../nope", false, &work, &s, false, &mut never),
                None
            );
            // --allow-dir already passed: consent given.
            assert_eq!(
                decide(true, tool, "../a", true, &work, &s, false, &mut never),
                None
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn root_symlink_inside_pointing_outside_is_outside() {
        let (_tmp, root, work) = tree();
        std::os::unix::fs::symlink(root.join("a"), work.join("escape")).unwrap();
        let log = std::cell::RefCell::new(Vec::new());
        let mut ask = recorder(&log, true);
        let s = store();
        assert_eq!(
            decide(true, "search", "escape", false, &work, &s, true, &mut ask),
            None
        );
        assert_eq!(
            *log.borrow(),
            vec![("search".to_string(), Approval::Tree(root.join("a")))],
            "canonicalized: the prompt and the key name the real directory"
        );
    }

    #[test]
    fn remembered_keys_are_exact_and_kinds_do_not_cover_each_other() {
        let (_tmp, root, work) = tree();
        std::fs::create_dir_all(root.join("a/deeper")).unwrap();
        let s = store();
        let log = std::cell::RefCell::new(Vec::new());
        let mut ask = recorder(&log, true);
        // A read of a file in `a` approves Files(a)...
        decide(
            true,
            "read_file",
            "../a/x",
            false,
            &work,
            &s,
            true,
            &mut ask,
        );
        // ...and a search whose root is a FILE in `a` rides it (parent key)...
        decide(true, "search", "../a/y", false, &work, &s, true, &mut ask);
        // ...but a recursive search ROOTED at `a` is a different grant.
        decide(true, "search", "../a", false, &work, &s, true, &mut ask);
        // A Tree yes for `a` covers list_files at `a` (same key)...
        decide(true, "list_files", "../a", false, &work, &s, true, &mut ask);
        // ...and not a root one level down.
        decide(
            true,
            "list_files",
            "../a/deeper",
            false,
            &work,
            &s,
            true,
            &mut ask,
        );
        assert_eq!(
            *log.borrow(),
            vec![
                ("read_file".to_string(), Approval::Files(root.join("a"))),
                ("search".to_string(), Approval::Tree(root.join("a"))),
                (
                    "list_files".to_string(),
                    Approval::Tree(root.join("a/deeper"))
                ),
            ]
        );
        // read_file is a file read even when handed a directory.
        assert_eq!(
            approval_for("read_file", &root.join("a")),
            Approval::Files(root.clone())
        );
    }
}
