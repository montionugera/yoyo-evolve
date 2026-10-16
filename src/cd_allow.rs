//! `[permissions] allow` entries suspended by `/cd` (#869 slice, Day 229).
//!
//! A project's `.yoyo.toml` can auto-approve bash commands (`allow = ["cargo *"]`)
//! once that project is trusted (#749). Those entries were written by THAT
//! project's author for THAT tree. Before this slice they were cloned into the
//! bash tool's confirm closure at build time and nothing on `/cd` touched them,
//! so after `/cd ../other`, `cargo build` in a tree the author never saw (its
//! `build.rs` included) still ran without asking — while the "trust is now OFF"
//! note printed by the same `/cd` said project `permissions.allow` "will not run
//! here".
//!
//! **Placement (d226): the drop lives in the authority, not beside one reader.**
//! [`crate::config::PermissionConfig::check`] reads the suspended list on every
//! call, so every caller of the check — the bash confirm closure, the file
//! confirm path, and any reader added later — gets the narrowing without being
//! wired. `check_with_suspended` is the pure seam tests drive.
//!
//! **Provenance is recorded where it is decided.** A session's permissions come
//! from exactly ONE source: `--allow`/`--deny` flags (which replace the config
//! wholesale), or the one config file `load_config_file` picked (first found
//! wins). So the launch project's entries are exactly the active allow list
//! when no flag was given and the winning file was the project-local one —
//! [`launch_project_allow_entries`] states that, and `cli.rs` records it once.
//!
//! **What `/cd` does with it:** outside the launch project's tree the launch
//! entries are suspended, except an entry the user's own config (`~/.yoyo.toml`,
//! `~/.config/yoyo/config.toml`) also lists — the user wrote that one for every
//! directory. Back inside the launch tree (the launch dir or any directory
//! below it) nothing is suspended, so `/cd` there and back is a round trip, not
//! a one-way door. The NEW directory's own `[permissions] allow` is still never
//! applied (it widens; the trust gate is a later slice).

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};

/// The launch project's allow entries and the directory they belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchProjectAllow {
    pub dir: PathBuf,
    pub entries: Vec<String>,
}

static LAUNCH_PROJECT_ALLOW: OnceLock<LaunchProjectAllow> = OnceLock::new();

static CD_SUSPENDED_ALLOW: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// A snapshot of the suspended list: what `PermissionConfig::check` and
/// `/permissions` read.
pub(crate) fn suspended_snapshot() -> Vec<String> {
    crate::sync_util::lock_or_recover(&CD_SUSPENDED_ALLOW).clone()
}

/// Which of the active allow entries came from the launch project's config.
///
/// Flags replace the config wholesale, so a flag session has none; a session
/// whose winning config is the user's own has none. Otherwise every active
/// allow entry is the project's (it only survived the #749 gate because the
/// project is trusted). Pure.
pub(crate) fn launch_project_allow_entries(
    from_cli_flags: bool,
    config_is_project_local: bool,
    active_allow: &[String],
) -> Vec<String> {
    if from_cli_flags || !config_is_project_local {
        return Vec::new();
    }
    active_allow.to_vec()
}

/// Record the launch project's entries once, at startup. First writer wins;
/// an empty list records nothing (there is nothing to suspend).
pub(crate) fn record_launch_project_allow(dir: &Path, entries: Vec<String>) {
    if entries.is_empty() {
        return;
    }
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let _ = LAUNCH_PROJECT_ALLOW.set(LaunchProjectAllow { dir, entries });
}

/// The startup consumer (`cli.rs`, right after the #749 gate): record against
/// the launch cwd. Provenance is read from the same single place the gate
/// reads it (`config::loaded_config_is_project_local`), never re-derived.
pub(crate) fn record_at_startup(from_cli_flags: bool, active: &[String]) {
    let project_local = crate::config::loaded_config_is_project_local();
    if let Ok(cwd) = std::env::current_dir() {
        let entries = launch_project_allow_entries(from_cli_flags, project_local, active);
        record_launch_project_allow(&cwd, entries);
    }
}

/// The entries to suspend once the session sits in `target`. Pure.
///
/// Inside the launch tree: none. Elsewhere: every launch entry the user's own
/// config does not also list.
pub(crate) fn suspended_allow_for(
    launch: Option<&LaunchProjectAllow>,
    target: &Path,
    user_allow: &[String],
) -> Vec<String> {
    let Some(launch) = launch else {
        return Vec::new();
    };
    if target.starts_with(&launch.dir) {
        return Vec::new();
    }
    launch
        .entries
        .iter()
        .filter(|e| !user_allow.contains(e))
        .cloned()
        .collect()
}

/// `[permissions] allow` from the user's own config files (both of them —
/// the user wrote each for every directory). Missing files contribute nothing.
fn user_config_allow() -> Vec<String> {
    let mut out = Vec::new();
    let paths = [
        crate::config::home_config_path(),
        crate::config::user_config_path(),
    ];
    for path in paths.into_iter().flatten() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for p in crate::config::parse_permissions_from_config(&content).allow {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// Set `store` to the suspension for `target` (replaced on every `/cd`, like
/// the Day-228 `[directories] allow` list). Returns the new list.
pub(crate) fn apply_suspension_with(
    store: &Mutex<Vec<String>>,
    launch: Option<&LaunchProjectAllow>,
    target: &Path,
    user_allow: &[String],
) -> Vec<String> {
    let target = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    let next = suspended_allow_for(launch, &target, user_allow);
    *crate::sync_util::lock_or_recover(store) = next.clone();
    next
}

/// The one-line note `/cd` prints when launch entries stop applying. `None`
/// when nothing is suspended — every session without a trusted project allow
/// list, which is the regression surface. Entries are repository-authored, so
/// they are sanitized for display.
pub(crate) fn suspension_note(suspended: &[String], launch_dir: &Path) -> Option<String> {
    if suspended.is_empty() {
        return None;
    }
    let n = suspended.len();
    let (noun, verb) = if n == 1 {
        ("pattern", "auto-approves")
    } else {
        ("patterns", "auto-approve")
    };
    let list: Vec<String> = suspended
        .iter()
        .map(|p| crate::cli::sanitize_for_display(p))
        .collect();
    Some(format!(
        "  {n} [permissions] allow {noun} from the launch project's .yoyo.toml no longer \
{verb} here: {}. Applies again if you /cd back under {}.",
        list.join(", "),
        crate::cli::sanitize_for_display(&launch_dir.display().to_string())
    ))
}

/// The `/cd` consumer: recompute the suspension for `target` and print its
/// note (unless quiet).
pub(crate) fn apply_cd_allow_suspension(target: &Path) {
    let launch = LAUNCH_PROJECT_ALLOW.get();
    let user_allow = if launch.is_some() {
        user_config_allow()
    } else {
        Vec::new()
    };
    // Day 230: the launch project's hooks pause by the same rule; their note
    // rides this call so dispatch.rs stays untouched.
    crate::cd_hooks::note_cd_hooks(target);
    let suspended = apply_suspension_with(&CD_SUSPENDED_ALLOW, launch, target, &user_allow);
    if let (Some(launch), false) = (launch, crate::format::is_quiet()) {
        if let Some(note) = suspension_note(&suspended, &launch.dir) {
            use crate::format::{DIM, RESET};
            eprintln!("{DIM}{note}{RESET}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PermissionConfig;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn launch() -> LaunchProjectAllow {
        LaunchProjectAllow {
            dir: PathBuf::from("/work/a"),
            entries: s(&["cargo *", "git status"]),
        }
    }

    #[test]
    fn provenance_flags_and_user_config_record_nothing() {
        let active = s(&["cargo *"]);
        // `--allow` flags replace the config: the entry is the user's, kept.
        assert!(launch_project_allow_entries(true, true, &active).is_empty());
        assert!(launch_project_allow_entries(true, false, &active).is_empty());
        // The winning config is the user's own: kept.
        assert!(launch_project_allow_entries(false, false, &active).is_empty());
        // Project-local winner: every active allow entry is the project's.
        assert_eq!(launch_project_allow_entries(false, true, &active), active);
    }

    #[test]
    fn launch_project_entry_is_suspended_after_cd_elsewhere() {
        let l = launch();
        assert_eq!(
            suspended_allow_for(Some(&l), Path::new("/work/b"), &[]),
            s(&["cargo *", "git status"])
        );
    }

    #[test]
    fn entry_also_in_user_config_keeps_applying() {
        let l = launch();
        assert_eq!(
            suspended_allow_for(Some(&l), Path::new("/work/b"), &s(&["cargo *"])),
            s(&["git status"])
        );
    }

    #[test]
    fn inside_the_launch_tree_nothing_is_suspended() {
        let l = launch();
        assert!(suspended_allow_for(Some(&l), Path::new("/work/a"), &[]).is_empty());
        assert!(suspended_allow_for(Some(&l), Path::new("/work/a/src"), &[]).is_empty());
        // Near miss: a sibling whose name merely starts with the launch dir's.
        assert_eq!(
            suspended_allow_for(Some(&l), Path::new("/work/ab"), &[]).len(),
            2
        );
    }

    #[test]
    fn no_launch_project_entries_means_no_suspension() {
        assert!(suspended_allow_for(None, Path::new("/work/b"), &[]).is_empty());
    }

    #[test]
    fn round_trip_restores_the_launch_entries() {
        let store = Mutex::new(Vec::new());
        let l = launch();
        let away = apply_suspension_with(&store, Some(&l), Path::new("/work/b"), &[]);
        assert_eq!(away.len(), 2);
        assert_eq!(*store.lock().unwrap(), away);
        let back = apply_suspension_with(&store, Some(&l), Path::new("/work/a"), &[]);
        assert!(back.is_empty());
        assert!(store.lock().unwrap().is_empty());
    }

    /// The emission point: what the bash/file confirm paths ask of the check.
    #[test]
    fn suspended_entry_no_longer_auto_approves_but_others_and_deny_still_apply() {
        let perms = PermissionConfig {
            allow: s(&["cargo *", "ls *"]),
            deny: s(&["rm *"]),
        };
        let suspended = s(&["cargo *"]);
        assert_eq!(perms.check_with_suspended("cargo build", &suspended), None);
        assert_eq!(perms.check_with_suspended("ls -la", &suspended), Some(true));
        assert_eq!(
            perms.check_with_suspended("rm -rf x", &suspended),
            Some(false)
        );
    }

    /// No /cd (empty suspension) is byte-identical to the pre-slice check.
    #[test]
    fn empty_suspension_is_identical_to_check() {
        let perms = PermissionConfig {
            allow: s(&["cargo *", "git status", "ls *"]),
            deny: s(&["rm *", "git push*"]),
        };
        for cmd in [
            "cargo build",
            "git status",
            "git push origin",
            "rm -rf /",
            "ls",
            "ls -la",
            "python x.py",
            "",
        ] {
            assert_eq!(
                perms.check_with_suspended(cmd, &[]),
                perms.check(cmd),
                "{cmd:?}"
            );
        }
    }

    #[test]
    fn note_names_count_entries_and_the_way_back() {
        assert_eq!(suspension_note(&[], Path::new("/work/a")), None);
        assert_eq!(
            suspension_note(&s(&["cargo *"]), Path::new("/work/a")).unwrap(),
            "  1 [permissions] allow pattern from the launch project's .yoyo.toml no longer \
auto-approves here: cargo *. Applies again if you /cd back under /work/a."
        );
        let two = suspension_note(&s(&["a", "b"]), Path::new("/w")).unwrap();
        assert!(
            two.starts_with("  2 [permissions] allow patterns from"),
            "{two}"
        );
    }
}
