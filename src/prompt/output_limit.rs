//! A reply cut off at the output-token limit says so on stderr.
//!
//! Before this module `StopReason::Length` was classified `Ignore`
//! (`prompt_retry::classify_stop_reason`) and nothing else in `src/` read it,
//! so `yoyo --max-tokens 30 -p "numbers 1 to 200"` printed `1 … 15`, exited 0
//! and wrote nothing to stderr: a truncated answer reported as a complete one.
//!
//! yoagent 0.25.2 does not continue on `Length`: `agent_loop.rs:727` maps an
//! assistant message stopped on `Length` to `RunEnd::Completed`, and
//! `extension.rs:459` says `on_stop` is not even called for it. So the cut-off
//! message is the turn's end, and the note keys on the turn's FINAL assistant
//! message only: a `Length` stop that was followed by more turns (tool calls
//! in a truncated message) is not what the user sees as the answer ending.
//!
//! What each door's reader can tell, which is why the mechanism is one stderr
//! line for all of them: `-p` into a pipe — the reader sees only stdout bytes
//! and cannot tell a cut-off from an ending; `-p` on a terminal and the REPL —
//! the human sees the text stop mid-sentence, but not why. Stdout stays
//! exactly as streamed, and the exit code stays 0 by decision: a user who sets
//! a low `max_tokens` on purpose would otherwise see every run fail.
//!
//! Volume is a warning (yellow, `warning:`), never DIM: a truncated answer is
//! not trivia. Not quiet-gated, like `max_tokens_ceiling_warning`.

use super::PromptEventState;
use crate::format::{is_plain_output, RESET, YELLOW};
use crate::prompt_retry::{classify_stop_reason, StopHandling};
use yoagent::{AgentMessage, Message};

/// True when the LAST assistant message of the run stopped on the output
/// limit. Reads through `classify_stop_reason` so the routing lives in one
/// place: `StopHandling::OutputLimit` is the `Length` arm.
pub(super) fn final_stop_hit_output_limit(messages: &[AgentMessage]) -> bool {
    messages
        .iter()
        .rev()
        .find_map(|m| match m {
            AgentMessage::Llm(Message::Assistant { stop_reason, .. }) => Some(stop_reason),
            _ => None,
        })
        .is_some_and(|r| classify_stop_reason(r) == StopHandling::OutputLimit)
}

/// The stderr line for a reply cut off at the output limit. Pure; colour is
/// added by the caller. Glyph-free when `plain`.
pub(crate) fn output_limit_note(max_tokens: Option<u32>, plain: bool) -> String {
    let limit = match max_tokens {
        Some(n) => format!("max_tokens = {n}"),
        None => "the model's default max_tokens".to_string(),
    };
    let marker = if plain { "" } else { "⚠ " };
    format!(
        "{marker}warning: reply stopped at the output limit ({limit}); \
         raise --max-tokens or max_tokens in .yoyo.toml"
    )
}

/// The AgentEnd step of `handle_prompt_events`: the existing handling first
/// (it flushes any held stdout text), then the note, so on a terminal the
/// warning lands after the last streamed byte.
pub(super) fn end_turn(
    state: &mut PromptEventState,
    messages: Vec<AgentMessage>,
    model: &str,
    max_tokens: Option<u32>,
) {
    let cut = final_stop_hit_output_limit(&messages);
    state.handle_agent_end(messages, model);
    if cut {
        let note = output_limit_note(max_tokens, is_plain_output());
        eprintln!("\n{YELLOW}  {note}{RESET}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoagent::{Content, StopReason, Usage};

    fn assistant(stop_reason: StopReason) -> AgentMessage {
        let text = vec![Content::Text {
            text: "1 2 3".into(),
        }];
        AgentMessage::Llm(Message::assistant(
            text,
            stop_reason,
            "m",
            "p",
            Usage::default(),
        ))
    }

    fn user(text: &str) -> AgentMessage {
        AgentMessage::Llm(Message::user(text))
    }

    #[test]
    fn output_limit_note_table() {
        let tail = "raise --max-tokens or max_tokens in .yoyo.toml";
        let cases = [
            (
                Some(30),
                true,
                format!("warning: reply stopped at the output limit (max_tokens = 30); {tail}"),
            ),
            (
                Some(30),
                false,
                format!("⚠ warning: reply stopped at the output limit (max_tokens = 30); {tail}"),
            ),
            (
                None,
                true,
                format!(
                    "warning: reply stopped at the output limit \
                     (the model's default max_tokens); {tail}"
                ),
            ),
            (
                None,
                false,
                format!(
                    "⚠ warning: reply stopped at the output limit \
                     (the model's default max_tokens); {tail}"
                ),
            ),
        ];
        for (max, plain, want) in cases {
            let got = output_limit_note(max, plain);
            assert_eq!(got, want, "max={max:?} plain={plain}");
            if plain {
                assert!(got.is_ascii(), "plain note must be glyph-free: {got:?}");
            }
        }
    }

    #[test]
    fn length_routes_to_the_output_limit_note_and_normal_ends_do_not() {
        assert_eq!(
            classify_stop_reason(&StopReason::Length),
            StopHandling::OutputLimit
        );
        assert!(final_stop_hit_output_limit(&[
            user("hi"),
            assistant(StopReason::Length)
        ]));
        // Near-miss: every normal ending stays silent, as before.
        for r in [StopReason::Stop, StopReason::ToolUse, StopReason::Aborted] {
            assert_ne!(classify_stop_reason(&r), StopHandling::OutputLimit);
            assert!(
                !final_stop_hit_output_limit(&[user("hi"), assistant(r.clone())]),
                "{r:?} must not warn"
            );
        }
        assert!(!final_stop_hit_output_limit(&[]));
    }

    #[test]
    fn only_the_final_assistant_message_counts() {
        // A Length stop followed by a later, normally-ended turn: the answer
        // the user sees ended normally, so no note.
        assert!(!final_stop_hit_output_limit(&[
            assistant(StopReason::Length),
            user("tool result"),
            assistant(StopReason::Stop),
        ]));
        // A trailing non-assistant message does not hide the final stop.
        assert!(final_stop_hit_output_limit(&[
            assistant(StopReason::Stop),
            user("tool result"),
            assistant(StopReason::Length),
            user("stop marker"),
        ]));
    }
}
