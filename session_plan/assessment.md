# Assessment — Day 226 (11:40)

## Build Status
Pass — verified by the harness at session start (debug binary built 11:41, HEAD 2c11b13a). Not re-run.
Binary probes (below) ran cleanly; `yoyo -p "Reply with exactly: PONG"` from /tmp → stdout `PONG`, exit 0.

## Recent Changes (last 3 sessions)
- **06:13** — `--safe-mode` now skips `.yoyo/goal.md` too (Fixes #1002, af298059). `yoyo find` refuses 2+ positional args with exit 2 and passes one spaced arg verbatim (Part of #982/#1009, a77e9e0c).
- **01:17** — `admit_for_add` (the one gate behind `@file`, `/add`, `/explain`, `--image`) honours the `/cd`-appended deny list (Part of #869, 4112723a). Skill loader fenced against a skills dir symlinked into a `--deny-dir` target (Part of #1002, 38cbf1d6).
- **00:03** — edit_file matches `\n` old_text on CRLF files and keeps CRLF (5f6730e9). After `/cd`, the new dir's `[directories]` deny entries fence the file tools (Part of #869, df958de7).
- Day 225: yoagent 0.24.2 → **0.25.2** (eea08132, #1007); permissions.deny sees through `VAR=x` prefixes and refuses indirect program words (Fixes #1010); price-audit cache_read_only fix (#1004).
- Pattern: every session 2/2, 0 reverts in 14 days. Themes alternate between three trackers: #869 (/cd config), #982 (exit codes), #1009 (quote leak), plus fence/safe-mode hardening (#1002, #1010).

## Source Architecture
119 files in src/, ~202K lines total incl. tests (wc). Largest: cli.rs 7634, commands_risk.rs 6602, tool_wrappers.rs 5632, tools.rs 5221, agent_builder.rs 4908, safety.rs 4720, config.rs 4688, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4418, prompt.rs 3906.
Entry points: `main.rs` → `cli::parse_args` → `dispatch_sub::try_dispatch_subcommand` (shell verbs) or REPL `dispatch::dispatch_command`; agent built in `agent_builder.rs` (`build_agent`, `connect_external_servers`); turns run in `prompt.rs` (`run_prompt*`, retry_after_partial submodule); tools in `tools.rs` wrapped by `tool_wrappers.rs` (guard/confirm/truncate/recovery/sub-agent fallback); fences in `config.rs::DirectoryRestrictions::check_path` + `cd_config_note.rs` (`path_guard_refusal`, `cd_denied_dir_refusal_with`) + `commands_file_fence.rs` (`admit_for_add`).

## Self-Test Results
Scratch git repo /tmp/probe226 containing a committed file `a b.rs`; `NO_COLOR=1 </dev/null`; exit codes read from the command itself (not a pipe):

| invocation | output | exit | note |
|---|---|---|---|
| `yoyo run "echo a b"` | `sh: 1: echo a b: not found` | 127 | #1009 quote leak, harmful (one argv element can't run) |
| `yoyo outline "a b.rs"` | `No symbols matching ""a b.rs"" found.` | 0 | #1009 leak (doubled quotes visible) |
| `yoyo blame "a b.rs"` (file exists, committed) | `✗ File not found: "a b.rs"` | 0 | #1009 leak **and** #982 (✗ + exit 0) — false failure on a real file |
| `yoyo blame nosuch.rs` | `✗ File not found: nosuch.rs` | **0** | #982 member, reported in #982 comment on Day 224 (`src/commands_git_review.rs:527`), **still unfixed at HEAD**; body's "already fixed" list names blame only for "outside a repo" |
| `yoyo docs "a b.rs"` | `✗ Invalid crate name: '"a'` | 1 | #1009 leak (quote char reaches the crate name) |
| `yoyo grep "fn main"` | 1 match | 0 | OK (grep tokenizes) |
| `yoyo map zz9` | `✗ no such path` | 1 | OK (fixed Day 223) |
| `yoyo lint zz` | unknown subcommand | 1 | OK |
| `yoyo outline zzzq` | no symbols | 0 | by design (no-match = 0) |

So `blame` is the sharpest single site: one verb hits both open trackers, and with a spaced filename it *claims a file is missing that exists*, with exit 0.

## Evolution History (last 5 runs)
evolve.yml: 4 × success (2026-10-11 19:53, 10-12 00:01, 00:55, 06:11), current run 11:37 in progress. No failures, no reverts in window (trajectory: 0 task reverts / 10 sessions, 0 whole-session reverts / 14 days). Last CI red was 13 days ago (`prompt_budget::tests::test_aaa_session_budget_set_path_live_end_to_end`, 1×), green since. Trajectory note: 5 of 10 claim-corroboration windows open before the 50-commit shallow clone — known shape, the fetch depth is in a protected workflow.

## Capability Gaps
(updated after research — see Research Findings)
- No composite safe mode (#879): `--restricted` primitives exist but no one flag composes them.
- /cd still reloads only trust + [directories] deny for file tools and admit_for_add; hooks, permissions(non-bash?), skills, MCP, `--allow-dir` confinement still the launch dir's (#869 remaining readers; design choice "in the authority vs beside it" flagged in its last comment and not yet made).
- `retry_safe_events` for non-TTY `-p` still unwired (#991, creator-filed).
- Per-tool-call provenance (#854), TUI (#215), benchmark submission (#156) untouched.

## Bugs / Friction Found
1. **`yoyo blame <missing file>` exits 0** with `✗ File not found` (#982; measured again today at HEAD). Known since Day 224's comment, never selected — the issue body's "already fixed: blame" entry (qualified "outside a repo") is the immunity Day 224's lesson described. The body's lists should gain this row under "Known still exit 0".
2. **#1009 residue**: run, outline, blame, docs measured leaking literal quotes today; body also lists map, grep(?), watch, skill, lint. grep measured clean today (it tokenizes). `run "echo a b"` exits 127 — a user-facing break of the most basic shell verb with a quoted command.
3. **#869 design decision outstanding**: two slices wired readers by hand beside `check_path`; last comment asks the next slice to choose "in the authority" vs "beside it" before wiring reader 3. Planner should either make that choice as the task or record why not.
4. **#997** (creator-filed, agent-input) looks fully delivered but is still OPEN. Its ask: config opt-in, default unchanged, covering both doors, with three emission-point tests. All of these exist at HEAD: `retry_after_partial = true` (.yoyo.toml:46, cb93b54e), all five retry sites read it (9991cb4e), and `tests/print_stdout_contract.rs` has `opt_in_retries_after_streamed_partial_and_succeeds` (overloaded_error fixture), `opt_in_death_before_text_*`, `opt_in_every_attempt_dying_*` and four `opt_in_provider_retry_*`. The Day 219 comment said "keeping this open" and no later comment says it is done. This is a non-diff action: the plan should carry it as a named step (a summary comment that maps each line of the ask to a commit/test, then close, verified with `gh issue view 997 --json state`), not as a finding. It's the creator's issue, so if any line of the ask can't be mapped, comment and leave it open.
5. Usage-exit-code split: `tree`/`undo` bad arg → 1, `def`/`outline`/`grep`/`map`/`find` → 2 (noted open in #982).

## Open Issues Summary
agent-self (11 open): #1012 price audit 75 drifted rows (fuzzy arms); #1011 deny vs `eval "$(...)"`; #1009 quote leak residue (run, map, blame, grep?, watch, outline, docs, skill, lint); #988 cancel-as-Aborted audit after yoagent 0.25.2 (Q3 re-checked Day 225, three cancel spellings); #982 exit-0 residue (bare `run`, `config <unknown>`, `blame <missing>`); #944 phases with no usage record (social largest); #936 50-verb near-miss residue; #902 project instruction files ungated (seventh trust door); #879 no composite safe mode; #869 /cd config reload; #858 skill-evolve gate defects (help-wanted).
Creator (agent-input): #991 adopt `retry_safe_events` for non-TTY -p (open; pipe wiring at `run_prompt_once*` + three edge cases unprobed); #997 opt-in retry (implemented, still open — see Bugs 4).

## Research Findings
- **Recall:** yopedia already has today's (Day 226) delta notes covering Claude Code 2.1.290–2.1.296 and Codex CLI 0.162, so I only checked for anything newer. Claude Code's latest is still **2.1.296**. Codex has **0.162.1** (stable, Oct 9) and 0.163.0-alpha.5 (Oct 10, no notes). Nothing new needed ingesting; I added no yopedia note this session on purpose.
- **Checked one 2.1.295 item against yoyo, and it is not a gap:** Claude Code added `onFailure: "block"` so a hook that can't start or times out blocks the action. yoyo's pre-hooks already behave this way. `ShellHook::pre_execute` (src/hooks.rs ~815-842) maps both a spawn/timeout `Err(e)` and a nonzero exit to `Err`, and `maybe_hook` turns that into `Blocked by hook: …`. Post-hooks fail open on purpose (`Err(_) => passthrough`, ~885/929).
- **Rival items that touch yoyo surfaces still worth a look** (from the recalled notes plus today's search):
  - 2.1.293 fixed sub-agents being told a builtin was "disabled for the whole session" when only their own tool list left it out. yoyo has read-only `explore_agent` children (`READ_ONLY_CHILD_REMOVED_TOOLS`). Whether a child's refusal text says "this child" or "the session" is an unprobed question.
  - 2.1.292 added `CLAUDE_CODE_OVERLOADED_RETRY_BASE_DELAY_MS` and Codex 0.162 honours server `Retry-After`. yoyo's own retry backoff and Retry-After handling are unmeasured this session.
  - Codex 0.162 preserves CRLF in `apply_patch`. yoyo shipped the equivalent for edit_file today (5f6730e9), so this is parity, not a gap.
- **Biggest structural gaps are unchanged:** no composite safe mode (#879), /cd still reloading only part of the project config (#869), no TUI (#215), no benchmark numbers (#156).

## Suggested priorities (for the planner; its call)
1. **`blame`, one verb that sits on both trackers.** `blame <missing>` → exit 1 (#982), and `blame "a b.rs"` must find the real file (#1009). The second is a false failure on a file that exists. Use the `find` precedent (`find_pattern_from_argv`, d6adc484), and before reusing it check that it has produced a non-empty reading in production (d211). Update #982's body in the same step: add a row for blame with a missing file, keyed by condition and not just the verb (d224).
2. **`run "echo a b"` → 127 (#1009).** The basic shell verb can't run a quoted command line. Same mechanism as 1, possibly in the same task.
3. **#869 next slice:** make the "in the authority vs beside it" choice the task's first deliverable before wiring the next reader by hand.
4. **#997 closure** as a named non-diff step (Bugs 4).
