//! The old-content read behind the non-verbose `write_file` diff preview
//! (#1052 item 3).
//!
//! The preview runs in `handle_tool_execution_start`, i.e. BEFORE the tool
//! does. It used to read the target with a bare `std::fs::read_to_string`
//! guarded only by `Path::exists()`, which is true for a FIFO, so a model
//! `write_file` aimed at a FIFO blocked the renderer on `open` forever and
//! yoagent's own `require_regular_file` refusal was never reached.
//!
//! Decision: a non-regular target (FIFO, socket, device) gets NO preview.
//! The tool is about to refuse it, and rendering it as a "new file" diff
//! (every line added) would be a false claim about what will happen. A
//! missing file, an empty path, a directory and an unreadable file also get
//! no preview, exactly as before; a regular file returns its content
//! unchanged, so its diff renders byte-identically.

use std::path::Path;

/// The current content of `path` for the `write_file` diff preview, or
/// `None` when there is nothing honest to diff against. Never opens a
/// non-regular file (one `metadata` call decides), so it cannot block.
pub(super) fn preview_old_content(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    crate::non_regular_read::read_regular_to_string(Path::new(path)).ok()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::non_regular_read::tests::make_fifo;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Run `f` on a thread and return its result, or `None` if it has not
    /// finished within `secs` (a regression then fails instead of hanging).
    fn within<T: Send + 'static>(secs: u64, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(Duration::from_secs(secs)).ok()
    }

    /// Anti-vacuous half: the fixture really is the hazard. The bare read the
    /// preview used before #1052 does not return on this FIFO. The blocked
    /// reader is then released by opening the FIFO for writing and closing it
    /// (the reader sees EOF), so no thread is leaked.
    #[test]
    fn bare_read_of_the_fifo_fixture_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let Some(fifo) = make_fifo(dir.path()) else {
            return;
        };
        let p = fifo.clone();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let r = std::fs::read_to_string(&p);
            let _ = tx.send(());
            r
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(500)).is_err(),
            "the bare read of a FIFO returned without a writer; the fixture no longer exercises #1052"
        );
        drop(std::fs::OpenOptions::new().write(true).open(&fifo).unwrap());
        reader.join().unwrap().unwrap();
    }

    #[test]
    fn fifo_target_gets_no_preview_and_returns_promptly() {
        let dir = tempfile::tempdir().unwrap();
        let Some(fifo) = make_fifo(dir.path()) else {
            return;
        };
        let s = fifo.to_str().unwrap().to_string();
        let got = within(5, move || preview_old_content(&s));
        assert_eq!(got, Some(None), "FIFO preview hung or rendered a diff");
    }

    #[test]
    fn regular_file_returns_its_exact_content() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        let body = "line one\nlíne ✓ two\n\nno trailing newline";
        std::fs::write(&f, body).unwrap();
        assert_eq!(
            preview_old_content(f.to_str().unwrap()),
            Some(body.to_string())
        );
    }

    #[test]
    fn missing_empty_and_directory_targets_get_no_preview() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.txt");
        assert_eq!(preview_old_content(missing.to_str().unwrap()), None);
        assert_eq!(preview_old_content(""), None);
        assert_eq!(preview_old_content(dir.path().to_str().unwrap()), None);
    }
}
