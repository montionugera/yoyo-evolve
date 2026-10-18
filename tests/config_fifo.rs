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
    let mut child = Command::new(env!("CARGO_BIN_EXE_yoyo"))
        .arg("permissions")
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
            panic!("yoyo permissions still running after {DEADLINE:?}: a FIFO .yoyo.toml hung it (#1047)");
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
