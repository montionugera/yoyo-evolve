//! #991: on a single-shot `-p` / piped run whose stdout is a pipe (not a
//! terminal, not reserved by `--print`/json), yoyo reads the agent's events
//! through yoagent's [`RetrySafeEvents`] filter, so an attempt yoagent retries
//! internally never reaches stdout at all. Bytes on a pipe cannot be taken
//! back; holding each attempt until it succeeds means there is nothing to take
//! back. The trade, named: text arrives once per assistant message instead of
//! token by token. A pipe has no one watching tokens arrive.
//!
//! What the filter does (yoagent 0.25.2 `retry.rs`, `docs/concepts/retry.md`):
//! - a successful attempt arrives whole (start, every update, end) at its end;
//! - a retried attempt disappears; only its `ProviderRetry` remains;
//! - an attempt that fails for good arrives as start + `Error`/`Aborted` end,
//!   WITHOUT its deltas;
//! - an attempt left open when the stream ends is dropped;
//! - every other event (tools, `ToolExecutionUpdate` from sub-agents) passes
//!   through at once, unfiltered.
//!
//! The third and fourth rows would lose text yoyo used to keep (a turn that
//! dies for good keeps its partial answer on stdout; Ctrl+C keeps what was
//! streamed, #993). So this gate shadows the raw text of the attempt in
//! flight: a final failure gets its text back as one synthetic text delta just
//! before its end, and Ctrl+C can flush whatever is still held.

use std::collections::VecDeque;
use std::io::IsTerminal;

use yoagent::retry::RetrySafeEvents;
use yoagent::{AgentEvent, AgentMessage, Message, StopReason, StreamDelta};

/// Should this run's text event loop hold attempts until they succeed?
/// Only single-shot runs (never the REPL), only an unreserved stdout (never
/// `--print`/json, which already hold the answer back), only a non-terminal
/// stdout (a terminal is unchanged: a human sees the restart).
pub(super) fn retry_safe_pipe_applies(
    single_shot: bool,
    stdout_reserved: bool,
    stdout_is_terminal: bool,
) -> bool {
    single_shot && !stdout_reserved && !stdout_is_terminal
}

/// Live-state wrapper of [`retry_safe_pipe_applies`].
pub(super) fn retry_safe_pipe_live() -> bool {
    retry_safe_pipe_applies(
        super::interrupt::is_single_shot(),
        crate::format::stdout_reserved(),
        std::io::stdout().is_terminal(),
    )
}

fn failed_assistant_end(event: &AgentEvent) -> Option<&AgentMessage> {
    match event {
        AgentEvent::MessageEnd { message } => match message {
            AgentMessage::Llm(Message::Assistant {
                stop_reason: StopReason::Error | StopReason::Aborted,
                ..
            }) => Some(message),
            _ => None,
        },
        _ => None,
    }
}

fn is_assistant_start(event: &AgentEvent) -> bool {
    matches!(
        event,
        AgentEvent::MessageStart {
            message: AgentMessage::Llm(Message::Assistant { .. })
        }
    )
}

/// The event source of the text loop: either the raw receiver (filter off,
/// byte-identical to before) or the raw receiver through [`RetrySafeEvents`].
pub(super) struct RetrySafeGate {
    filter: Option<RetrySafeEvents>,
    queue: VecDeque<AgentEvent>,
    closed: bool,
    /// Raw text of the attempt currently open (held by the filter).
    open_text: String,
    /// Raw text of an attempt that ended with Error/Aborted and is held until
    /// the next event says whether it was retried.
    failed_text: String,
}

impl RetrySafeGate {
    pub(super) fn new(filtering: bool) -> Self {
        Self {
            filter: filtering.then(RetrySafeEvents::new),
            queue: VecDeque::new(),
            closed: false,
            open_text: String::new(),
            failed_text: String::new(),
        }
    }

    /// Is the filter on? (Off: every event is passed through untouched.)
    pub(super) fn is_filtering(&self) -> bool {
        self.filter.is_some()
    }

    /// Feed one raw event; queue whatever the filter releases. Released final
    /// failures get their shadowed text back as a synthetic delta first.
    /// `None` = the raw stream ended.
    fn feed(&mut self, raw: Option<AgentEvent>) {
        let Some(filter) = self.filter.as_mut() else {
            self.queue.extend(raw);
            return;
        };
        let released = match &raw {
            Some(event) => filter.push(event.clone()),
            None => filter.finish(),
        };
        for event in released {
            if let Some(message) = failed_assistant_end(&event) {
                let text = std::mem::take(&mut self.failed_text);
                if !text.is_empty() {
                    self.queue.push_back(AgentEvent::MessageUpdate {
                        message: message.clone(),
                        delta: StreamDelta::Text { delta: text },
                    });
                }
            }
            self.queue.push_back(event);
        }
        // Shadow bookkeeping AFTER the push: a held failure is released by the
        // NEXT event, so its text must still be in `failed_text` at that point.
        match raw {
            Some(AgentEvent::MessageUpdate {
                delta: StreamDelta::Text { delta },
                ..
            }) => self.open_text.push_str(&delta),
            Some(ref event) if is_assistant_start(event) => self.open_text.clear(),
            Some(ref event) if failed_assistant_end(event).is_some() => {
                self.failed_text = std::mem::take(&mut self.open_text);
            }
            Some(AgentEvent::MessageEnd { .. }) => self.open_text.clear(),
            Some(AgentEvent::ProviderRetry { .. }) => {
                // The retried attempt disappeared; so does its text.
                self.open_text.clear();
                self.failed_text.clear();
            }
            None => self.closed = true,
            _ => {}
        }
    }

    /// Next event for the loop. Cancel-safe for `tokio::select!`: the only
    /// await is `rx.recv()`, and state changes happen after it returns.
    pub(super) async fn next(
        &mut self,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<AgentEvent>,
    ) -> Option<AgentEvent> {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return Some(event);
            }
            if self.closed {
                return None;
            }
            let raw = rx.recv().await;
            if self.filter.is_none() {
                return raw;
            }
            self.feed(raw);
        }
    }

    /// Ctrl+C: text the filter is still holding (an attempt in flight, or a
    /// failed one awaiting its verdict), so the interrupt keeps what the model
    /// already produced (#993). Empty when the filter is off.
    pub(super) fn take_held_text(&mut self) -> String {
        let mut text = std::mem::take(&mut self.failed_text);
        text.push_str(&std::mem::take(&mut self.open_text));
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoagent::Usage;

    fn assistant(stop_reason: StopReason) -> AgentMessage {
        AgentMessage::Llm(Message::assistant(
            Vec::new(),
            stop_reason,
            "m",
            "p",
            Usage::default(),
        ))
    }

    fn start() -> AgentEvent {
        AgentEvent::MessageStart {
            message: assistant(StopReason::Stop),
        }
    }
    fn text(t: &str) -> AgentEvent {
        AgentEvent::MessageUpdate {
            message: assistant(StopReason::Stop),
            delta: StreamDelta::Text { delta: t.into() },
        }
    }
    fn end(reason: StopReason) -> AgentEvent {
        AgentEvent::MessageEnd {
            message: assistant(reason),
        }
    }
    fn retry() -> AgentEvent {
        AgentEvent::provider_retry(1, 3, "slow down", std::time::Duration::ZERO)
    }

    /// Text deltas the loop would print, in order, after the stream ends.
    fn printed(gate: &mut RetrySafeGate, events: Vec<AgentEvent>) -> Vec<String> {
        for e in events {
            gate.feed(Some(e));
        }
        gate.feed(None);
        gate.queue
            .drain(..)
            .filter_map(|e| match e {
                AgentEvent::MessageUpdate {
                    delta: StreamDelta::Text { delta },
                    ..
                } => Some(delta),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn only_a_single_shot_unreserved_pipe_is_filtered() {
        // (single_shot, reserved, tty) -> filter?
        let table = [
            (true, false, false, true),   // -p | cat: the door #991 is about
            (true, false, true, false),   // -p on a terminal: unchanged
            (true, true, false, false),   // --print / json: already held back
            (false, false, false, false), // REPL with stdout piped: unchanged
            (false, false, true, false),
            (true, true, true, false),
        ];
        for (single, reserved, tty, want) in table {
            assert_eq!(
                retry_safe_pipe_applies(single, reserved, tty),
                want,
                "single={single} reserved={reserved} tty={tty}"
            );
        }
    }

    #[test]
    fn retried_attempt_text_never_reaches_the_loop() {
        let mut gate = RetrySafeGate::new(true);
        let out = printed(
            &mut gate,
            vec![
                start(),
                text("PARTIAL_"),
                end(StopReason::Error),
                retry(),
                start(),
                text("PONG"),
                end(StopReason::Stop),
            ],
        );
        assert_eq!(out, vec!["PONG".to_string()]);
    }

    #[test]
    fn final_failure_keeps_its_partial_once() {
        let mut gate = RetrySafeGate::new(true);
        let out = printed(
            &mut gate,
            vec![
                start(),
                text("PART"),
                text("IAL_"),
                end(StopReason::Error),
                retry(),
                start(),
                text("PARTIAL_"),
                end(StopReason::Error),
            ],
        );
        // Anti-vacuous: the retried copy is gone, the final one survives once.
        assert_eq!(out, vec!["PARTIAL_".to_string()]);
    }

    #[test]
    fn held_text_is_available_to_an_interrupt() {
        let mut gate = RetrySafeGate::new(true);
        gate.feed(Some(start()));
        gate.feed(Some(text("ABC")));
        assert!(gate.queue.is_empty(), "the open attempt is held");
        assert_eq!(gate.take_held_text(), "ABC");
        assert_eq!(gate.take_held_text(), "", "taken once");
    }

    #[test]
    fn filter_off_passes_every_event_through_unchanged() {
        let mut gate = RetrySafeGate::new(false);
        let out = printed(
            &mut gate,
            vec![
                start(),
                text("PARTIAL_"),
                end(StopReason::Error),
                retry(),
                start(),
                text("PONG"),
                end(StopReason::Stop),
            ],
        );
        assert_eq!(out, vec!["PARTIAL_".to_string(), "PONG".to_string()]);
        assert_eq!(gate.take_held_text(), "");
    }
}
