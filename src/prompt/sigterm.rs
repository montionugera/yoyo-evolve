//! #1015: SIGTERM on a single-shot run whose stdout is a pipe.
//!
//! On that population the retry-safe gate (#991) HOLDS each attempt's text
//! until its `MessageEnd`, so the default SIGTERM disposition (die at once)
//! lost it: `timeout -s TERM 12 yoyo -p ... | wc -c` read 0 bytes while the
//! same run with `-s INT` read 4920. Callers: any CI / `timeout` /
//! supervisor wrapper around `yoyo -p | ...`, and evolve.sh's own
//! `timeout $t yoyo ... | tee log` phases.
//!
//! **The hazard this module is shaped around:** once tokio registers a
//! SIGTERM handler, the default disposition is gone for the rest of the
//! process. A SIGTERM arriving while no `select!` polls it would be
//! swallowed and `timeout` would have to escalate to SIGKILL. So:
//! - the handler is registered only when the gate is filtering (TTY, REPL,
//!   `--print`, json and stream-json never register it and still die by
//!   default), and
//! - the handler is a process-lifetime watcher task that ALWAYS ends the
//!   process with 143: when a text loop is listening it hands the loop the
//!   signal (the loop flushes and returns) and exits after a grace if `main`
//!   has not already; when no loop is listening it flushes stdout and exits.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;
use tokio::sync::watch;

/// 128 + SIGTERM(15), the shell convention (SIGINT is 130).
pub(super) const SIGTERM_EXIT: i32 = 143;

static TERMINATED: AtomicBool = AtomicBool::new(false);
/// Text loops currently polling a [`SigtermListener`].
static LISTENING: AtomicUsize = AtomicUsize::new(0);
static CHANNEL: OnceLock<watch::Sender<bool>> = OnceLock::new();

/// Whether a SIGTERM was received by this process (#1015: exit 143).
pub(super) fn was_terminated() -> bool {
    TERMINATED.load(Ordering::SeqCst)
}

/// A text loop's handle on the SIGTERM watcher; dropping it tells the
/// watcher no loop will flush, so it must end the process itself.
pub(super) struct SigtermListener {
    rx: watch::Receiver<bool>,
}

impl Drop for SigtermListener {
    fn drop(&mut self) {
        LISTENING.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Arm the watcher (once per process) and return a listener — only when
/// `filtering` (the population whose text is held). `None` everywhere else,
/// and on non-unix targets, so those runs never replace the default.
pub(super) fn listen(filtering: bool) -> Option<SigtermListener> {
    if !filtering || !cfg!(unix) {
        return None;
    }
    let tx = CHANNEL.get_or_init(install);
    LISTENING.fetch_add(1, Ordering::SeqCst);
    Some(SigtermListener { rx: tx.subscribe() })
}

/// Resolve when SIGTERM arrives; never resolves for `None` (unarmed).
pub(super) async fn recv(listener: &mut Option<SigtermListener>) {
    match listener.as_mut() {
        Some(l) => {
            let _ = l.rx.wait_for(|terminated| *terminated).await;
        }
        None => std::future::pending::<()>().await,
    }
}

#[cfg(unix)]
fn install() -> watch::Sender<bool> {
    use std::io::Write;
    use std::time::Duration;
    use tokio::signal::unix::{signal, SignalKind};
    /// How long the listening loop gets to flush and `main` to exit 143.
    const LOOP_GRACE: Duration = Duration::from_secs(5);
    let (tx, _) = watch::channel(false);
    // Registration failure leaves the default disposition intact.
    if let Ok(mut sig) = signal(SignalKind::terminate()) {
        tokio::spawn(async move {
            if sig.recv().await.is_none() {
                return;
            }
            TERMINATED.store(true, Ordering::SeqCst);
            if LISTENING.load(Ordering::SeqCst) > 0 {
                if let Some(tx) = CHANNEL.get() {
                    tx.send_replace(true);
                }
                // A second SIGTERM skips the grace.
                tokio::select! {
                    _ = tokio::time::sleep(LOOP_GRACE) => {}
                    _ = sig.recv() => {}
                }
            } else {
                let note = super::interrupt::interrupt_note(true);
                let _ = std::io::stderr().write_all(note.as_bytes());
            }
            let _ = std::io::stdout().flush();
            std::process::exit(SIGTERM_EXIT);
        });
    }
    tx
}

#[cfg(not(unix))]
fn install() -> watch::Sender<bool> {
    watch::channel(false).0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The population guard: a run whose gate does not hold text (TTY, REPL,
    /// --print, json, stream-json) never registers a handler, so SIGTERM
    /// keeps its default disposition there.
    #[test]
    fn unfiltered_runs_never_register_a_sigterm_handler() {
        assert!(listen(false).is_none());
    }
}
