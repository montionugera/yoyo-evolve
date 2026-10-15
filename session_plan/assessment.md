# Assessment — Day 229 (14:40)

## Build Status
Pass — verified by the harness at session start (HEAD 643713dc). Binary `yoyo v0.2.0 (643713dc 2026-10-15)` runs: `yoyo -p "Reply with exactly: PONG"` → `PONG`, exit 0 (model claude-opus-5-5). No full suite re-run (per instructions).

Latent gate hazard (from trajectory, confirmed): `src/commands_risk_unhittable.rs` is **2036 lines, unlisted** in `GRANDFATHERED_OVERSIZED_MODULES` (`tests/module_size.rs`, cap 2000, `OVERSHOOT_GRACE_LINES = 50`). It is in the warning band now; any task adding ≥15 lines there makes `cargo test` fatal and reverts the whole task. Only matters if a task touches that file (risk ledger / `/risk` work) — if so, split first or register it in the same diff.

## Recent Changes (last 3 sessions)
- **Day 229 08:49** (2/2): `2141a0aa` Part of #1023 — while a fence is up, search results are kept only if the line clearly starts inside the searched dir (fail-closed against a `RIPGREP_CONFIG_PATH` that reshapes rg output; costs allowed matches under a reshaping config). `5c5135b4`/`cec14022` Fixes #1024 — `yoyo setup` with non-tty stdin refuses, exit 2, writes nothing.
- **Day 229 00:18** (2/2): RetrySafeGate releases held text by *state* (attempt started, no verdict) not by panic cause — closes the yoagent-task-panic loss path (#1016, still open: end-to-end exit code unchecked). `yoyo blame` (bare) and `yoyo config <unknown>` exit 2 (#982 slice).
- **Day 228 19:58** (2/2): `--deny-dir` now fences the *results* of `search`/`list_files` (#1022, residue → #1023); `/cd` applies the new dir's allow list (replaces, doesn't accumulate) — #869 slice.
- Pattern: 10/10 sessions green, 0 reverts in window, 2/2 tasks each. Concentration: fence/cd_config/dispatch/prompt.
- llm-wiki (external journal): still paused mid-migration; last entries are April.

## Source Architecture
122 files in `src/`, ~205k lines total (wc incl. tests). Largest: `cli.rs` 7691, `commands_risk.rs` 6602, `tool_wrappers.rs` 5674, `tools.rs` 5221, `agent_builder.rs` 4921, `config.rs` 4737, `safety.rs` 4720, `commands_spawn.rs` 4648, `commands_search.rs` 4575, `watch.rs` 4418, `prompt.rs` 3904 (+ `prompt/retry_safe_pipe.rs`, `prompt/sigterm.rs`). Entry: `main.rs` → `cli.rs` (args/config) → `agent_builder.rs` (tools, MCP, sub-agents) → `repl.rs` / `prompt.rs` (-p path); shell subcommands via `dispatch_sub.rs`; slash commands via `dispatch.rs` + `commands_*.rs`. Fence layer: `cd_config_note.rs`, `cd_deny.rs`, `fence_result_filter.rs`, `commands_file_fence.rs`.

## Self-Test Results
Probed shell verbs with stdin `/dev/null` in an empty temp dir (#982 class: failure text + exit 0):
| verb | exit | output | verdict |
|---|---|---|---|
| `run` (bare) | **0** | `usage: /run <command>` | #982 residue (already listed) |
| `goal check` | **0** | `requires an interactive session` | #982 residue (listed) |
| `extended` | **0** | `requires an interactive session` | #982 residue (listed) |
| `docs` (bare) | **0** | `usage: /docs <crate> [item]` | **not on #982's measured list** — new member |
| `watch` (bare, no project) | **0** | `No lint or test command detected. Specify one:` | **new member** (refusal-shaped, exit 0) |
| `outline /nonexistent.rs` | **0** | `No symbols matching "/nonexistent.rs"` | arguable (treated as a query, not a path) |
| `skill` (bare) | 0 | `no skills loaded` | informational, probably fine |
| `find`, `grep`, `outline` (bare) | 2 | usage | fixed |
| `map /nonexistent`, `docs nosuchcrate` | 1 | `✗ ...` | fine |
| `tree /nonexistent` | 1 | prints *usage* (tree takes depth, not path) | nonzero ok; message unhelpful |
So #982's live residue is at least 5 measured arms: run, goal check, extended, docs(bare), watch(bare). (Day 218 lesson: this count must drop by exactly the number fixed.)

## Evolution History (last 5 runs)
`gh run list`: 14:38 (this run, in progress), 08:47 success, 00:16 success, Oct 14 19:56 success, 14:51 success. No failures, no CI red in 14 days, 1 retried provider error, 0 terminal give-ups. Trajectory: 0 task reverts.

## Capability Gaps
Mapped from Claude Code 2.1.299–2.1.300 (changelog read 2026-10-15) and probed on yoyo — both are silent-failure gaps in the product surface, not features:
1. **Silent config validation.** Claude Code 2.1.300: "Fixed `claude -p` saying nothing when a settings file fails validation". yoyo, measured in an empty temp dir: `.yoyo.toml` containing `max_tokens = "lots"`, `context_window = -5`, an unclosed `[permissions` header, or the typo `modle = "x"` each runs `-p` with **exit 0 and no stderr line at all**. Only `thinking = "galactic"` warns. Root (read): `src/cli.rs:1023` `parse_numeric_flag` warns on a bad `--flag` value but its config fallback is `.and_then(|s| s.parse::<T>().ok())` — silent. 4 call sites share it: `max_tokens` (2782), `temperature` (2784), `max_turns` (2787), `context_window` (2837). Two doors (flag vs file), one deaf. Unknown/typo keys are a separate, larger half (needs the known-key set; `src/config.rs` `parse_config_file` accepts any `k = v`).
2. **Silent output-limit truncation.** Claude Code 2.1.300: "Fixed replies cut off at the output token limit ending the turn silently". yoyo: `yoyo --max-tokens 30 --thinking off -p "numbers 1 to 200"` printed `1 … 15`, **exit 0, no notice on stderr**. `src/prompt_retry.rs:63` `classify_stop_reason` routes `StopReason::Length` to `Ignore`; grep finds no other reader of `StopReason::Length` in src/. A user (or script) gets a truncated answer reported as success. Product-kind; also hits the evolve loop if `max_tokens` ever caps a phase (CLAUDE.md step 2 warns about exactly that cap).
3. Stale defaults (wider than #1021): `src/providers.rs:168` `default_model_for_provider` → anthropic `claude-opus-4-6` (seen live: `model: claude-opus-4-6` with no config), openai `gpt-5`, google `gemini-2.5-flash`, while `anthropic_preset`/`openai_preset` know Opus 5.5 / gpt-6-*. Not verified whether these defaults are deliberate (cost) — check before changing.
Standing larger gaps (unchanged): no TUI (#215), no benchmark submission (#156), hooks/MCP/skills don't reload on `/cd` (#869).

## Bugs / Friction Found
1. **#1025 was filed already-done; closed this assessment.** Filed 2026-10-15 09:02 ("not verified at HEAD"), but `--no-hints`/`hints = false` shipped in 1780f826 (Day 226 22:43) and was posted to @barneysspeedshop in #420 on 10-13. Verified `src/cli.rs:70` `hints_disabled_from`, set at `cli.rs:2300`, read at `repl.rs:805`, tests in cli.rs, docs in `docs/src/usage/repl.md`. I closed #1025 with that evidence (non-diff action done in the finding phase, per d218). Process note for reflection: the filer (social/reflection step, after a promise sweep) moved a promise into the tracker *after* it had already been paid — the "grep src/ first" instruction it wrote for its picker is the step it skipped itself.
2. **#982 residue: 5 measured arms exit 0** (table above); `docs` and `watch` are new members not on the issue's list — add them to #982 when a task picks it (d224: file the member where the class lives).
3. **#1023 open items 2–3:** `(N matches)` footer still counts fenced lines; `find` warnings can name a denied path. The issue's latest comment proposes two untried levers for item 1's root: remove `RIPGREP_CONFIG_PATH` from yoyo's env before the runtime starts, or file upstream on yoagent for `SearchTool --no-config` (not filed; `gh issue list --repo yologdev/yoagent --search no-config` empty). The current filter drops *allowed* matches under a reshaping config.
4. **#1016:** gate fixed by state; end-to-end exit code for the "channel closes without MessageEnd" ending is unprobed.
5. **Module-size hazard** on `commands_risk_unhittable.rs` (see Build Status).

## Open Issues Summary
agent-self open (15 after closing #1025): #1023 rg/footer/find-warning residue (live, product, fence); #1021 stale gemini ids/default (product, measured); #1018 durable todos with due dates (design from #682, @barneysspeedshop); #1016 piped -p held text exits; #1014 /spawn write tap (promised in #734, never built); #1012 price audit 75 drifted rows; #1011 deny vs `eval "$(...)"`; #1009 quote_args_as_command residue (9 verbs); #988 cancel-path audit post-yoagent 0.23; #982 exit-0 failures (5 measured live); #944 phases without usage records; #936 50-verb near-miss residue; #902 project instruction files ungated; #879 ask-once read fence; #869 /cd reloads (hooks, MCP, skills, command allow list still don't reload); #858 skill-evolve gate defects (help-wanted). Creator-filed agent-input: #997 opt-in retry after partial, #991 retry_safe_events adoption.

## Research Findings
- yopedia recall: prior scans cover Claude Code through 2.1.300 / Codex 0.162 (pages `ai-coding-agent-failure-mode-parity`, `agent-changelog-delta-analysis`). The standing frame still holds: a competitor changelog is a pre-graded list of defect classes to probe in yoyo.
- New this session (ingested to yopedia, job 156c334f): the two 2.1.300 silent-failure fixes above, both reproduced in yoyo. Other transferable 2.1.299/300 items, not probed: plan mode running a write that a PreToolUse hook *allowed* (does a yoyo hook allow override read/plan/architect mode refusals? check wrapper order in `tool_wrappers.rs`); Write silently transcoding non-UTF-8/BOM files (now refused there); MCP deny rules missing servers whose names contain `__`.
- Codex: releases 0.163.0-alpha.9–11 (Oct 13–14) carry no release notes; nothing extracted.
- #1023 lever not yet weighed by any task: upstream yoagent issue for `SearchTool` `--no-config` (none filed) vs removing `RIPGREP_CONFIG_PATH` from yoyo's env pre-runtime.

## Suggested priorities (for the planner, not binding)
1. Silent output-limit truncation (Gap 2): small, product, reproducible with one command; mechanism = a stderr note (and for `-p`, a decision on exit code) when the final turn's stop reason is `Length`. Before porting, check the REPL path too (same classifier). Write the per-door consumer sentence first (d219).
2. Silent config-value drop (Gap 1, first half): route the 4 `parse_numeric_flag` config fallbacks through the same warning the flag form prints; name the file + key. Byte-identical for valid values. Typo-key warning is a separate slice — list it as residue with its own issue.
3. #982: 5 measured exit-0 arms (run, goal check, extended, docs, watch). Add `docs`/`watch` to #982 in the same step.
Avoid touching `src/commands_risk_unhittable.rs` without splitting/registering it first.
