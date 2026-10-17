//! `read_file` refuses FIFOs, devices and sockets instead of hanging (#1047).
//!
//! yoagent 0.25.7's `ReadFileTool` trusts `metadata.len()` for its size check,
//! and that is `0` for a FIFO, a tty or a character device, so the check passes
//! and `read_to_string` runs: on a FIFO or tty it blocks forever, on
//! `/dev/zero` it grows without bound. An unattended `-p` run just hangs until
//! it is killed. Nobody who reaches that branch chose it, so the refusal is
//! unconditional — it runs with or without `--allow-dir`/`--deny-dir`, because
//! `GuardedTool`/`ArcGuardedTool` wrap every `read_file` unconditionally.
//!
//! What it does NOT touch, so those paths stay byte-identical:
//! - **Missing paths / metadata errors** → `None`; the inner tool reports its
//!   own `Cannot access …` error rather than a second invented message.
//! - **Directories** → `None`; yoagent already fails fast and clearly
//!   (`Cannot read <dir>: Is a directory (os error 21)`), so nothing hangs.
//! - **Symlinks to regular files** → `None`; `std::fs::metadata` follows links.
//!
//! The path is checked as given, which is the path yoagent opens: with no
//! sandbox roots configured, `PathSandbox::io_path` returns it unchanged and it
//! resolves against the process cwd, exactly as `std::fs::metadata` does.

use std::path::Path;

/// The refusal for a non-regular, non-directory file, or `None` to let the
/// real tool run. Pure apart from one `metadata` call; never opens the file.
pub(crate) fn non_regular_file_refusal(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let ft = meta.file_type();
    if ft.is_file() || ft.is_dir() {
        return None;
    }
    Some(format!(
        "{} is not a regular file ({}): reading it could block forever or never end; \
         read_file only reads regular files",
        path.display(),
        kind_name(&ft)
    ))
}

#[cfg(unix)]
fn kind_name(ft: &std::fs::FileType) -> &'static str {
    use std::os::unix::fs::FileTypeExt;
    if ft.is_fifo() {
        "FIFO"
    } else if ft.is_char_device() {
        "character device"
    } else if ft.is_block_device() {
        "block device"
    } else if ft.is_socket() {
        "socket"
    } else {
        "special file"
    }
}

#[cfg(not(unix))]
fn kind_name(_ft: &std::fs::FileType) -> &'static str {
    "special file"
}

/// The wiring seam for the path-argument guards: only `read_file` is checked.
pub(crate) fn check(tool: &str, path: &str) -> Option<String> {
    if tool != "read_file" {
        return None;
    }
    non_regular_file_refusal(Path::new(path))
}

#[cfg(all(test, unix))]
pub(crate) mod tests {
    use super::*;
    use std::os::unix::fs::FileTypeExt;

    /// Make a FIFO in `dir`, or `None` if `mkfifo` is unavailable.
    pub(crate) fn make_fifo(dir: &Path) -> Option<std::path::PathBuf> {
        let p = dir.join("pipe.txt");
        let ok = std::process::Command::new("mkfifo")
            .arg(&p)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        ok.then_some(p)
    }

    #[test]
    fn fifo_is_refused_and_named() {
        let dir = tempfile::tempdir().unwrap();
        let Some(fifo) = make_fifo(dir.path()) else {
            eprintln!("mkfifo unavailable; skipping");
            return;
        };
        // Anti-vacuous: the fixture really is a FIFO.
        assert!(std::fs::metadata(&fifo).unwrap().file_type().is_fifo());
        let msg = non_regular_file_refusal(&fifo).expect("FIFO must be refused");
        assert!(msg.contains("FIFO"), "{msg}");
        assert!(msg.contains("read_file only reads regular files"), "{msg}");
    }

    #[test]
    fn char_device_is_refused() {
        let p = Path::new("/dev/null");
        assert!(std::fs::metadata(p).unwrap().file_type().is_char_device());
        let msg = non_regular_file_refusal(p).expect("/dev/null must be refused");
        assert!(msg.contains("device"), "{msg}");
    }

    #[test]
    fn regular_file_passes() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hi\n").unwrap();
        assert_eq!(non_regular_file_refusal(&f), None);
    }

    #[test]
    fn symlink_to_regular_file_passes() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hi\n").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&f, &link).unwrap();
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(non_regular_file_refusal(&link), None);
    }

    #[test]
    fn missing_path_and_directory_pass_through() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(non_regular_file_refusal(&dir.path().join("nope")), None);
        assert_eq!(non_regular_file_refusal(dir.path()), None);
    }

    /// Emission point: the wrapped `read_file` exactly as `tools.rs` builds it
    /// (`maybe_guard`/`maybe_guard_arc` over `ReadFileTool::default()`, NO fence
    /// configured). A hang is the bug, and `tokio::time::timeout` cannot catch
    /// it: the stuck read lives in `spawn_blocking`, which a `#[tokio::test]`
    /// runtime waits for at shutdown. So each call runs on a detached thread
    /// with its own runtime and the test waits with `recv_timeout`; a regression
    /// panics by name and the leaked thread dies with the test process.
    fn run_wrapped_read(arc: bool, path: std::path::PathBuf) -> Result<String, String> {
        use std::sync::Arc;
        use yoagent::tools::file::ReadFileTool;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let none = crate::cli::DirectoryRestrictions::default();
            let params = serde_json::json!({ "path": path.to_str().unwrap() });
            let ctx = yoagent::types::ToolContext::new("c1", "read_file");
            let out = rt.block_on(async {
                if arc {
                    crate::tool_wrappers::maybe_guard_arc(Arc::new(ReadFileTool::default()), &none)
                        .execute(params, ctx)
                        .await
                } else {
                    crate::tool_wrappers::maybe_guard(Box::new(ReadFileTool::default()), &none)
                        .execute(params, ctx)
                        .await
                }
            });
            let mapped = match out {
                Ok(r) => Ok(r
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        yoagent::types::Content::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<String>()),
                Err(yoagent::types::ToolError::Failed(m)) => Err(m),
                Err(e) => Err(format!("other error: {e:?}")),
            };
            let _ = tx.send(mapped);
        });
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("read_file (arc={arc}) hung: #1047 regressed"))
    }

    #[test]
    fn wrapped_read_file_refuses_fifo_without_hanging_both_wrappers() {
        let dir = tempfile::tempdir().unwrap();
        let Some(fifo) = make_fifo(dir.path()) else {
            eprintln!("mkfifo unavailable; skipping");
            return;
        };
        for arc in [false, true] {
            match run_wrapped_read(arc, fifo.clone()) {
                Err(m) => assert!(m.contains("FIFO"), "arc={arc}: {m}"),
                Ok(t) => panic!("arc={arc}: expected a FIFO refusal, got content {t:?}"),
            }
        }
    }

    /// Near-miss at the same seam: a regular file still reads, content intact.
    #[test]
    fn wrapped_read_file_still_reads_a_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hello fifo-free world\n").unwrap();
        for arc in [false, true] {
            let text = run_wrapped_read(arc, f.clone()).expect("regular file must read");
            assert!(text.contains("hello fifo-free world"), "arc={arc}: {text}");
        }
    }

    #[test]
    fn check_only_applies_to_read_file() {
        assert!(check("read_file", "/dev/null").is_some());
        assert_eq!(check("list_files", "/dev/null"), None);
        assert_eq!(check("write_file", "/dev/null"), None);
    }
}
