//! A FIFO at `.yoyo.toml` must not hang startup (#1047).
//!
//! Opening a FIFO for reading blocks until a writer appears, so a cloned repo
//! carrying one hung every yoyo invocation in it (measured Day 232: `yoyo -p`
//! exit 124 under `timeout 12`, no output). The loader now skips a non-regular
//! project config with one stderr warning and keeps walking the chain to the
//! user config. These tests drive the real binary at the emission point:
//! `yoyo permissions`, which reads config through `load_config_file` and makes
//! no network call, and prints the `[permissions] deny` entries it loaded.
//! (`yoyo config get` was tried first and rejected: it reads the file through
//! its own `detect_loaded_config_path` door, not `load_config_file`.)
//! Never invokes cargo.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const DEADLINE: Duration = Duration::from_secs(10);

/// Run `yoyo permissions` in `cwd` with HOME/XDG pointed at `home`, and
/// return its output, or panic if it is still running after `DEADLINE`.
fn permissions(cwd: &Path, home: &Path) -> Output {
    run_yoyo(&["permissions"], cwd, home)
}

/// Run `yoyo <args>` like [`permissions`]: panics (rather than hanging the
/// suite) if it is still running after `DEADLINE`.
fn run_yoyo(args: &[&str], cwd: &Path, home: &Path) -> Output {
    run_yoyo_env(args, cwd, home, &[])
}

/// [`run_yoyo`] with extra environment variables (same deadline).
fn run_yoyo_env(args: &[&str], cwd: &Path, home: &Path, env: &[(&str, &str)]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_yoyo"))
        .args(args)
        .envs(env.iter().copied())
        .current_dir(cwd)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env_remove("MODEL")
        .env_remove("YOYO_MODEL")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn yoyo");
    let start = Instant::now();
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            return child.wait_with_output().expect("collect output");
        }
        if start.elapsed() > DEADLINE {
            let _ = child.kill();
            let _ = child.wait();
            panic!("yoyo {args:?} still running after {DEADLINE:?}: a FIFO hung it (#1047, #1052)");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn mkfifo(path: &Path) -> bool {
    Command::new("mkfifo")
        .arg(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Control: the subcommand really reads the project `.yoyo.toml`. Without
/// this, every FIFO test below could pass against a command that never opens it.
#[test]
fn control_regular_project_config_is_read() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        proj.path().join(".yoyo.toml"),
        "[permissions]\ndeny = [\"fifo-test-pattern\"]\n",
    )
    .unwrap();
    let out = permissions(proj.path(), home.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("fifo-test-pattern"),
        "control failed: stdout={stdout:?} stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Near-miss: a symlink to a regular config still loads.
#[test]
fn symlinked_regular_project_config_still_loads() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let real = proj.path().join("real.toml");
    std::fs::write(&real, "[permissions]\ndeny = [\"linked-pattern\"]\n").unwrap();
    std::os::unix::fs::symlink(&real, proj.path().join(".yoyo.toml")).unwrap();
    let out = permissions(proj.path(), home.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("linked-pattern"), "stdout={stdout:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("not a regular file"), "stderr={stderr:?}");
}

#[test]
fn fifo_project_config_does_not_hang_and_is_named() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let fifo = proj.path().join(".yoyo.toml");
    if !mkfifo(&fifo) {
        eprintln!("mkfifo unavailable; skipping");
        return;
    }
    {
        use std::os::unix::fs::FileTypeExt;
        // Anti-vacuous: the fixture really is a FIFO.
        assert!(std::fs::metadata(&fifo).unwrap().file_type().is_fifo());
    }
    let out = permissions(proj.path(), home.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(".yoyo.toml"), "stderr={stderr:?}");
    assert!(stderr.contains("not a regular file"), "stderr={stderr:?}");
    assert!(stderr.contains("FIFO"), "stderr={stderr:?}");
}

#[test]
fn fifo_project_config_falls_through_to_user_config() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        home.path().join(".yoyo.toml"),
        "[permissions]\ndeny = [\"user-pattern\"]\n",
    )
    .unwrap();
    if !mkfifo(&proj.path().join(".yoyo.toml")) {
        eprintln!("mkfifo unavailable; skipping");
        return;
    }
    let out = permissions(proj.path(), home.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("user-pattern"),
        "stdout={stdout:?} stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Near-miss for the `config get` row: a regular `.yoyo.toml` is still read,
/// and the value and its source are printed exactly (`key = value  (source)`). Without this, the FIFO test below could
/// pass against a `config get` that never opens the file.
#[test]
fn config_get_reads_a_regular_project_config() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(proj.path().join(".yoyo.toml"), "model = \"m\"\n").unwrap();
    let out = run_yoyo(&["config", "get", "model"], proj.path(), home.path());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "model = m  (.yoyo.toml)",
        "stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("not a regular file"), "stderr={stderr:?}");
}

/// `yoyo config get` and `yoyo config show` against a FIFO `.yoyo.toml`
/// exit on their own (run_yoyo panics if not) and name the file. Both used
/// to block in `read_to_string` until killed (exit 124 under `timeout 5`).
#[test]
fn config_get_and_show_do_not_hang_on_a_fifo_project_config() {
    for args in [&["config", "get", "model"][..], &["config", "show"][..]] {
        let proj = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let fifo = proj.path().join(".yoyo.toml");
        if !mkfifo(&fifo) {
            eprintln!("mkfifo unavailable; skipping");
            return;
        }
        let out = run_yoyo(args, proj.path(), home.path());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.code().is_some(),
            "{args:?} was killed by a signal: {:?}",
            out.status
        );
        assert!(stderr.contains(".yoyo.toml"), "{args:?} stderr={stderr:?}");
        assert!(
            stderr.contains("not a regular file"),
            "{args:?} stderr={stderr:?}"
        );
    }
}

/// Near-miss for #1052 item 1: a regular `system_file` named by the project
/// `.yoyo.toml` is still read, and its text is the system prompt.
#[test]
fn regular_config_system_file_is_still_read() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(proj.path().join(".yoyo.toml"), "system_file = \"sp\"\n").unwrap();
    std::fs::write(proj.path().join("sp"), "SYSFILE_TOKEN_1052\n").unwrap();
    let out = run_yoyo(&["--print-system-prompt"], proj.path(), home.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SYSFILE_TOKEN_1052"),
        "stdout={stdout:?} stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// #1052 item 1: a FIFO named by the project config's `system_file` used to
/// block startup in `read_to_string` (exit 124 under `timeout 10`). It now
/// fails like any other unreadable `system_file`: an error naming the file,
/// exit 1, before any model call.
#[test]
fn fifo_config_system_file_does_not_hang_and_is_named() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::write(proj.path().join(".yoyo.toml"), "system_file = \"sp\"\n").unwrap();
    let fifo = proj.path().join("sp");
    if !mkfifo(&fifo) {
        eprintln!("mkfifo unavailable; skipping");
        return;
    }
    {
        use std::os::unix::fs::FileTypeExt;
        assert!(std::fs::metadata(&fifo).unwrap().file_type().is_fifo());
    }
    let out = run_yoyo(&["--print-system-prompt"], proj.path(), home.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr={stderr:?}");
    assert!(stderr.contains("system_file 'sp'"), "stderr={stderr:?}");
    assert!(stderr.contains("not a regular file"), "stderr={stderr:?}");
}

/// #1052 item 2: a FIFO at the `--continue` session path used to block in
/// `read_to_string` (exit 124 under `timeout 10`). The lenient restore now
/// warns, names the reason, and carries on; `--continue-strict` refuses.
/// The model call afterwards fails fast against a closed local port.
#[test]
fn fifo_continue_session_file_does_not_hang() {
    for (flag, strict) in [("--continue", false), ("--continue-strict", true)] {
        let proj = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let fifo = proj.path().join("yoyo-session.json");
        if !mkfifo(&fifo) {
            eprintln!("mkfifo unavailable; skipping");
            return;
        }
        let out = run_yoyo_env(
            &[flag, "-p", "hi"],
            proj.path(),
            home.path(),
            &[
                ("ANTHROPIC_API_KEY", "dummy"),
                ("ANTHROPIC_BASE_URL", "http://127.0.0.1:1"),
            ],
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("yoyo-session.json"), "{flag}: {stderr:?}");
        assert!(stderr.contains("not a regular file"), "{flag}: {stderr:?}");
        assert!(
            !stderr.contains("no previous session found"),
            "{flag}: a FIFO is not an absent session: {stderr:?}"
        );
        if strict {
            assert_eq!(out.status.code(), Some(1), "{flag}: {stderr:?}");
        }
    }
}

/// Near-miss for item 2: a missing session file still reads as "no previous
/// session found", byte-identical to before.
#[test]
fn missing_continue_session_file_still_reports_no_session() {
    let proj = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_yoyo_env(
        &["--continue", "-p", "hi"],
        proj.path(),
        home.path(),
        &[
            ("ANTHROPIC_API_KEY", "dummy"),
            ("ANTHROPIC_BASE_URL", "http://127.0.0.1:1"),
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no previous session found (yoyo-session.json)"),
        "stderr={stderr:?}"
    );
    assert!(!stderr.contains("not a regular file"), "stderr={stderr:?}");
}
