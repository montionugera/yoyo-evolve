//! The launch project's shell hooks pause after `/cd` leaves its tree (#869
//! slice, Day 230).
//!
//! A hook in `<repo A>/.yoyo.toml` is a command repo A's author wrote to run
//! against repo A's tree. Before this, `/cd ../B` left every such hook firing on
//! B's tool calls (probed: a `hooks.post.bash = "pwd >> probe"` in A wrote
//! `.../A`, then `.../B` after `/cd ../B`). Suspending them only narrows, so it
//! needs no trust decision. The new directory's own hooks are still NOT loaded
//! (that widens; the trust gate is a later slice).
//!
//! **Provenance.** A session's shell hooks come from ONE file, the one
//! `load_config_file` picked (no flag adds hooks), so either every hook is
//! project-authored or none is. `record_at_startup` reads the same
//! `config::loaded_config_is_project_local()` the #820 trust gate reads and, when
//! it is project-local, records the identity of every hook that passed the gate.
//! A user-level config (`~/.yoyo.toml`, XDG) records nothing, so its hooks never
//! pause. Identity, not a session-wide flag, so a later merge of user and project
//! hooks would still pause only the project's.
//!
//! **Placement (d226): in the authority.** `ShellHook`'s three phase entries
//! (`pre_execute`, `post_execute`, `post_failure_execute` — the only callers of
//! `run_command`, 3) each consult `is_suspended`, so every registry and every
//! future caller is narrowed without being wired. It is keyed on STATE (the
//! process cwd at the moment the hook would fire), not on the `/cd` event, so a
//! cwd change that does not go through `/cd` is covered too; `/cd` only prints
//! the note.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::hooks::ShellHook;

/// What makes two hooks "the same hook": phase, pattern and command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookId {
    pub phase: &'static str,
    pub tool_pattern: String,
    pub command: String,
}

impl HookId {
    pub(crate) fn of(hook: &ShellHook) -> Self {
        HookId {
            phase: hook.phase.as_str(),
            tool_pattern: hook.tool_pattern.clone(),
            command: hook.command.clone(),
        }
    }
}

/// The launch project's hooks and the (canonical) directory they belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchProjectHooks {
    pub dir: PathBuf,
    pub hooks: Vec<HookId>,
}

static LAUNCH_PROJECT_HOOKS: OnceLock<LaunchProjectHooks> = OnceLock::new();

/// Whether the last `/cd` note reported the hooks as paused, so the next `/cd`
/// back can say they fire again. Only the NOTE reads this; the runtime check
/// reads the cwd.
static NOTED_SUSPENDED: AtomicBool = AtomicBool::new(false);

/// Which hooks are the launch project's. Empty unless the loaded config was
/// project-local. Pure.
pub(crate) fn launch_project_hooks(
    config_is_project_local: bool,
    active: &[ShellHook],
) -> Vec<HookId> {
    if !config_is_project_local {
        return Vec::new();
    }
    active.iter().map(HookId::of).collect()
}

/// The startup consumer (`cli.rs`, right after the #820 gate). First writer
/// wins; nothing to suspend records nothing.
pub(crate) fn record_at_startup(active: &[ShellHook]) {
    let hooks = launch_project_hooks(crate::config::loaded_config_is_project_local(), active);
    if hooks.is_empty() {
        return;
    }
    if let Ok(cwd) = std::env::current_dir() {
        let dir = std::fs::canonicalize(&cwd).unwrap_or(cwd);
        let _ = LAUNCH_PROJECT_HOOKS.set(LaunchProjectHooks { dir, hooks });
    }
}

/// Is the launch tree left? `None` cwd (unreadable) counts as left: we cannot
/// say we are still inside the project, so its hooks do not run (fail closed).
/// Pure.
pub(crate) fn outside_launch_tree(launch: &LaunchProjectHooks, cwd: Option<&Path>) -> bool {
    match cwd {
        Some(cwd) => !cwd.starts_with(&launch.dir),
        None => true,
    }
}

/// Does `hook` pause with the session sitting in `cwd`? Pure.
pub(crate) fn hook_suspended_at(
    launch: Option<&LaunchProjectHooks>,
    cwd: Option<&Path>,
    hook: &HookId,
) -> bool {
    let Some(launch) = launch else {
        return false;
    };
    launch.hooks.contains(hook) && outside_launch_tree(launch, cwd)
}

fn canonical_cwd() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    Some(std::fs::canonicalize(&cwd).unwrap_or(cwd))
}

/// The runtime check every `ShellHook` phase entry makes.
pub(crate) fn is_suspended(hook: &ShellHook) -> bool {
    let Some(launch) = LAUNCH_PROJECT_HOOKS.get() else {
        return false; // the common case: no project hooks recorded, no cwd read
    };
    hook_suspended_at(Some(launch), canonical_cwd().as_deref(), &HookId::of(hook))
}

/// The `/cd` note. `Some` on a transition only: paused (from running) or
/// running again (from paused). Pure.
pub(crate) fn cd_hook_note(
    launch: &LaunchProjectHooks,
    was_suspended: bool,
    now_suspended: bool,
) -> Option<String> {
    let n = launch.hooks.len();
    let dir = crate::cli::sanitize_for_display(&launch.dir.display().to_string());
    let (noun, verb_stop, verb_fire) = if n == 1 {
        ("hook", "no longer fires", "fires")
    } else {
        ("hooks", "no longer fire", "fire")
    };
    match (was_suspended, now_suspended) {
        (false, true) => Some(format!(
            "  {n} {noun} from the launch project's .yoyo.toml {verb_stop} here. \
             {} again if you /cd back under {dir}.",
            if n == 1 { "Fires" } else { "Fire" }
        )),
        (true, false) => Some(format!(
            "  {n} {noun} from the launch project's .yoyo.toml {verb_fire} again (back under {dir})."
        )),
        _ => None,
    }
}

/// The `/cd` consumer (called from `cd_allow::apply_cd_allow_suspension`, so
/// `dispatch.rs` is untouched): print the note on a transition, unless quiet.
pub(crate) fn note_cd_hooks(target: &Path) {
    let Some(launch) = LAUNCH_PROJECT_HOOKS.get() else {
        return;
    };
    let target = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    let now = outside_launch_tree(launch, Some(&target));
    let was = NOTED_SUSPENDED.swap(now, Ordering::SeqCst);
    if crate::format::is_quiet() {
        return;
    }
    if let Some(note) = cd_hook_note(launch, was, now) {
        use crate::format::{DIM, RESET};
        eprintln!("{DIM}{note}{RESET}");
    }
}

/// The `/hooks` marker for a hook that is paused right now, else `None`.
pub(crate) fn hooks_listing_marker(suspended: bool) -> Option<&'static str> {
    suspended.then_some(
        "suspended: a launch-project hook, and the session has /cd'd outside that project",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::HookPhase;

    fn hook(phase: HookPhase, pattern: &str, cmd: &str) -> ShellHook {
        ShellHook {
            name: format!("{}:{pattern}", phase.as_str()),
            phase,
            tool_pattern: pattern.to_string(),
            command: cmd.to_string(),
        }
    }

    fn launch(dir: &str, hooks: &[ShellHook]) -> LaunchProjectHooks {
        LaunchProjectHooks {
            dir: PathBuf::from(dir),
            hooks: hooks.iter().map(HookId::of).collect(),
        }
    }

    #[test]
    fn only_a_project_local_config_records_hooks() {
        let hs = [hook(HookPhase::Post, "bash", "pwd >> /tmp/x")];
        assert_eq!(launch_project_hooks(true, &hs), vec![HookId::of(&hs[0])]);
        // A user-level config's hooks are the user's, for every directory.
        assert!(launch_project_hooks(false, &hs).is_empty());
    }

    #[test]
    fn no_launch_record_means_nothing_ever_pauses() {
        // Byte-identical behaviour for every session without project hooks.
        let h = HookId::of(&hook(HookPhase::Pre, "*", "true"));
        for cwd in [Some(Path::new("/work/b")), Some(Path::new("/")), None] {
            assert!(!hook_suspended_at(None, cwd, &h));
        }
    }

    #[test]
    fn transition_table_away_back_and_subdirectory() {
        let project = hook(HookPhase::Post, "bash", "pwd >> /tmp/x");
        let l = launch("/work/a", std::slice::from_ref(&project));
        let id = HookId::of(&project);
        let cases: &[(Option<&str>, bool)] = &[
            (Some("/work/a"), false),          // never moved
            (Some("/work/a/src/deep"), false), // subdirectory: same verdict as staying put
            (Some("/work/b"), true),           // another project
            (Some("/work/ab"), true),          // a sibling sharing the prefix is NOT under /work/a
            (Some("/work"), true),             // the parent is outside too
            (None, true),                      // cwd unreadable: fail closed
        ];
        for (cwd, want) in cases {
            assert_eq!(
                hook_suspended_at(Some(&l), cwd.map(Path::new), &id),
                *want,
                "cwd {cwd:?}"
            );
        }
        // Round trip: away then back restores (state-keyed, nothing latches).
        assert!(hook_suspended_at(Some(&l), Some(Path::new("/work/b")), &id));
        assert!(!hook_suspended_at(
            Some(&l),
            Some(Path::new("/work/a")),
            &id
        ));
    }

    #[test]
    fn a_hook_that_is_not_the_launch_projects_keeps_firing_outside() {
        // Near-miss: a hook not recorded as project-authored (e.g. a user hook,
        // if hooks were ever merged) must not pause, even outside the tree.
        let project = hook(HookPhase::Post, "bash", "pwd >> /tmp/x");
        let user = hook(HookPhase::Post, "bash", "echo user");
        let l = launch("/work/a", std::slice::from_ref(&project));
        let b = Some(Path::new("/work/b"));
        assert!(hook_suspended_at(Some(&l), b, &HookId::of(&project)));
        assert!(!hook_suspended_at(Some(&l), b, &HookId::of(&user)));
        // Same command, different phase: a different hook.
        let other_phase = hook(HookPhase::Pre, "bash", "pwd >> /tmp/x");
        assert!(!hook_suspended_at(Some(&l), b, &HookId::of(&other_phase)));
    }

    #[test]
    fn note_fires_on_transitions_only() {
        let one = launch("/work/a", &[hook(HookPhase::Post, "bash", "x")]);
        assert_eq!(
            cd_hook_note(&one, false, true).unwrap(),
            "  1 hook from the launch project's .yoyo.toml no longer fires here. Fires again \
             if you /cd back under /work/a."
        );
        assert_eq!(
            cd_hook_note(&one, true, false).unwrap(),
            "  1 hook from the launch project's .yoyo.toml fires again (back under /work/a)."
        );
        assert_eq!(cd_hook_note(&one, false, false), None);
        assert_eq!(cd_hook_note(&one, true, true), None);
        let two = launch(
            "/work/a",
            &[
                hook(HookPhase::Post, "bash", "x"),
                hook(HookPhase::Pre, "*", "y"),
            ],
        );
        assert_eq!(
            cd_hook_note(&two, false, true).unwrap(),
            "  2 hooks from the launch project's .yoyo.toml no longer fire here. Fire again \
             if you /cd back under /work/a."
        );
    }

    #[test]
    fn note_sanitizes_the_launch_dir() {
        let l = launch("/work/\x1b[31ma", &[hook(HookPhase::Post, "bash", "x")]);
        // Anti-vacuous: the fixture carries the hostile byte.
        assert!(l.dir.display().to_string().as_bytes().contains(&0x1b));
        let note = cd_hook_note(&l, false, true).unwrap();
        assert!(!note.as_bytes().contains(&0x1b));
    }

    #[test]
    fn listing_marks_suspended_hooks_and_leaves_running_ones_alone() {
        assert!(hooks_listing_marker(true).unwrap().starts_with("suspended"));
        assert_eq!(hooks_listing_marker(false), None);
    }
}
