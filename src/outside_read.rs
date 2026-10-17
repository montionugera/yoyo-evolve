//! Opt-in ask-once fence for `read_file` outside the working directory
//! (part of #879).
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
//! **Ask once per directory.** A yes is remembered for the rest of the session
//! for the target's parent directory, exactly, so three files in `/etc` ask
//! once. It does not cover subdirectories: a yes for your home directory must
//! not quietly grant `~/.ssh`. A no refuses this read only and is not
//! remembered, so the next read in that directory asks again.
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
//! **Not a sandbox.** This fences the `read_file` tool only. `bash` can still
//! `cat` the same file, and `search`/`list_files` are not covered yet (residue
//! on #879). `--restricted` and `--allow-dir` remain the hard fences.
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

/// The one tool this slice fences.
const FENCED_TOOL: &str = "read_file";

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Directories approved this session (canonical parent directories).
static APPROVED_DIRS: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

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

/// The directory an approval is remembered for.
fn approval_dir(target: &Path) -> PathBuf {
    target
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| target.to_path_buf())
}

/// The refusal a non-interactive run gets.
fn no_tty_refusal(target: &Path, cwd: &Path) -> String {
    format!(
        "{ENV_VAR}=1{stem}read_file refused '{}': it is outside the working directory '{}', \
         and there is no terminal to ask on. To allow it, unset {ENV_VAR}, run yoyo from a \
         directory that contains the file, or name its directory with --allow-dir.",
        target.display(),
        cwd.display(),
        stem = crate::tool_wrappers::REFUSAL_STEM_MODE_ACTIVE,
    )
}

/// The refusal an interactive "no" gets.
fn declined_refusal(target: &Path) -> String {
    format!(
        "{}read_file on '{}' (outside the working directory)",
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
    store: &Mutex<HashSet<PathBuf>>,
    is_tty: bool,
    ask: &mut dyn FnMut(&Path, &Path) -> bool,
) -> Option<String> {
    if !enabled || tool != FENCED_TOOL || explicit_allow {
        return None;
    }
    let target = resolve_outside(path, cwd)?;
    let dir = approval_dir(&target);
    // Held across the prompt so two parallel reads cannot both ask.
    let mut approved = crate::sync_util::lock_or_recover(store);
    if approved.contains(&dir) {
        return None;
    }
    if !is_tty {
        return Some(no_tty_refusal(&target, cwd));
    }
    if ask(&target, &dir) {
        approved.insert(dir);
        None
    } else {
        Some(declined_refusal(&target))
    }
}

/// Ask on stderr, read one line from stdin.
fn ask_on_terminal(target: &Path, dir: &Path) -> bool {
    let target = crate::cli::sanitize_for_display(&target.display().to_string());
    let dir = crate::cli::sanitize_for_display(&dir.display().to_string());
    eprint!(
        "{YELLOW}  ⚠ read_file wants '{target}', outside the working directory. Allow reads in {dir} this session? {RESET}({GREEN}y{RESET}/{RED}n{RESET}) "
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
        &APPROVED_DIRS,
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

    fn store() -> Mutex<HashSet<PathBuf>> {
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
        let mut ask = |_: &Path, _: &Path| {
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
        let mut ask = |_: &Path, dir: &Path| {
            asked.borrow_mut().push(dir.to_path_buf());
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
        let mut no = |_: &Path, _: &Path| {
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
        let msg = no_tty_refusal(Path::new("/etc/hosts"), cwd);
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
        let mut ask = |_: &Path, _: &Path| -> bool { panic!("must not prompt without a tty") };
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
        assert_eq!(out, Some(no_tty_refusal(&root.join("outside.txt"), &work)));
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
        let mut ask = |_: &Path, _: &Path| -> bool { panic!("must not prompt") };
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
        // Other tools are out of this slice.
        for tool in ["write_file", "list_files", "search"] {
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
}
