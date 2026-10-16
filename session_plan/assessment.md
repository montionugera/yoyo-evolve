# Assessment — Day 230 (19:18)

## Build Status
Pass, verified by the harness at session start for HEAD 7964e5fa. Binary `target/debug/yoyo` reports `yoyo v0.2.0 (7964e5fa 2026-10-16)`. In an empty dir with no config, `yoyo -p "Reply with exactly PONG"` printed `model: claude-opus-5-5`, answered `PONG` and exited 0, so the 14:31 #1037 anthropic-default fix holds. I did not re-run the full suite.

## Recent Changes (last 3 sessions)
- **14:31**: e7a33c5b, Part of #1037. The no-config anthropic default moved `claude-opus-4-6` → `claude-opus-5-5`, and `--help` now derives the default from `default_model_for_provider`. 2b5a5c17, Part of #982: bare `yoyo watch` with nothing detected now exits 2.
- **11:14**: Fixes #1036. A malformed `.yoyo.toml` is refused with exit 2 and file:line. `config get` and `doctor` still only warn. Part of #869: `/cd` home skips deny entries already enforced, and `/permissions` no longer lists patterns twice.
- **08:44**: Fixes #1032. `/cd` back to the launch dir no longer warns "NOT applied". Fixes #1034: an unterminated quote or unclosed header warns with file:line, wired into three readers.
- **07:14**: one task reverted. #1033's done-when named the protected `skills/evolve/SKILL.md`, and it was bundled with #1032. Yuanhao then wrote #1033 himself. yoagent 0.25.2 → 0.25.5 (b58e4329).
- llm-wiki (external journal) is still "paused mid-migration" in every entry. No new external work.

## Source Architecture
125 files in src/, ~207k lines (wc over src/*.rs + src/*/*.rs). Largest: cli.rs 7688, commands_risk.rs 6602, tool_wrappers.rs 5674, tools.rs 5221, agent_builder.rs 4921, config.rs 4758, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4471, prompt.rs 3905. Entry points: main.rs → cli::parse_args → agent_builder (build_agent, connect_external_servers) → prompt.rs (run_prompt*) / repl. Shell verbs go through dispatch_sub.rs::try_dispatch_subcommand. Tool guards are in tool_wrappers.rs. Fence logic: cd_config_note.rs, cd_deny.rs, cd_allow.rs, cd_hooks.rs, fence_result_filter.rs.

## Self-Test Results
Shell-verb exit-code probe, run from an empty non-git dir with stdin /dev/null:
- `watch lint` (no lint command detected): prints the "No lint command detected" usage and **exits 0**. This is the sibling #982 already names as "not fixed, read from code", now **measured**. Its twin, bare `watch`, was fixed to exit 2 at 14:31.
- Bare `docs`: prints `usage: /docs <crate> [item]` and **exits 0**. Bare `grep`, `find`, `def` and `outline` print usage and exit 2. Bare `docs` is not on #982's "exit 0 by design" list, so it's an unlisted member of the same class.
- `tree /nonexistent` prints the usage and exits 1. #982 records that as fixed ("tree <bad arg>", exit 1). Usage errors elsewhere are 2, so this is a minor inconsistency.
- `outline nofile.rs` prints "No symbols matching ..." and exits 0. That's arguably a no-match, which is the open grep-convention decision in #982, not a bug.
- `skill`, `todo`, `risk` (bare) exit 0 with informational text. That's fine.
- `grep hello "d sp"` (spaced path, a #1009 verb) works: 1 match, exit 0.

**Stale model-facing claim found (#1023's last open item, still unfixed at HEAD):** `src/fence_result_filter.rs:18-21` says yoyo "cannot add `--no-config`; `rg` reads whatever `RIPGREP_CONFIG_PATH` names". `unattributed_note()` (~line 109), a string the **model receives** when the search fence drops lines, says "(a ripgrep config via RIPGREP_CONFIG_PATH can reshape output)". yoagent 0.25.5, which is pinned in Cargo.lock, passes `--no-config` and clears `RIPGREP_CONFIG_PATH`/`GREP_OPTIONS` for the search child. So both sentences have been false since b58e4329, and today's lesson ("the one claim the upgrade falsified had no marker") is still unpaid in the code. ARCHITECTURE.md's entry at line 1350 needs the same correction. A fix should re-read what the fail-closed rule still protects against now that rg ignores configs: grep fallback with `GREP_OPTIONS` is cleared too, but `--json`-style reshaping is no longer reachable through env. Keep the fail-closed behaviour if any shape remains (for example, filenames containing `:`), and rewrite the reason.

## Evolution History (last 5 runs)
gh run list (evolve.yml): 14:29, 11:12, 08:42, 07:12 all **success**. 19:15 is this run, in progress. Trajectory: 9 of the last 10 sessions were 2/2. One (08:30) was 1/2 with "did not reach a verdict", which is the protected-file revert. No CI failures and no provider errors in 14 days. **Concentration warning from the trajectory: config took 5 of the last 9 self-driven diffs, and cd_config 4/9. Send this session's self-driven slot to a different subsystem.**

## Capability Gaps
- **Binary files through read_file.** Claude Code 2.1.301 fixed Read treating .xlsm/.parquet/.epub/.heic as text. yoagent 0.25.5's `read_file` (`src/tools/file.rs:184`) uses `read_to_string`, so a non-UTF-8 binary returns the raw `Cannot read X: stream did not contain valid UTF-8`. That is an error, not garbage, but it doesn't tell the model "this is a binary file". A valid-UTF-8 file with NULs would read as text. Not probed. If worth anything, it's an upstream ask (`--label upstream`), not a yoyo rebuild.
- **A cancel reported as a failure.** CC 2.1.301 #19: a tool call cut off mid-turn was reported to the model as "rejected by user". That's the same class as yoyo's open #988 (Aborted / on_error not called). The rival is still fixing this class too.
- **Codex 0.163:** parallel read-only tools (yoagent already has `ToolExecutionStrategy::Parallel`), a verified sandbox launcher for local MCP servers, network policy enforced in MCP commands. No new gap that yoyo can't already reach. The persistent big gaps (TUI #215, benchmarks #156) are unchanged.

## Bugs / Friction Found
1. **Stale claim in model-facing text** (fence_result_filter.rs, above). Subsystem: tools/fence, not config. Small, product-facing, and pays off today's archived lesson. Part of #1023. Also check whether #1023's items 2 (the `(N matches)` footer counts withheld lines) and 3 (`find` warning lines pass denied paths verbatim) are still true under 0.25.5: its search footer and `details.engine` changed.
2. **`yoyo watch lint` exits 0 on "No lint command detected"** (measured). This is the named sibling of today's 2b5a5c17 fix, so it's a one-arm change in watch.rs/dispatch_sub.rs. Part of #982.
3. **Bare `yoyo docs` exits 0 on its usage line** (measured, and not on #982's lists). Add it to #982's body in place, by condition ("bare verb with required arg → usage → 2").
4. yoagent **0.25.6** was released today (13:51Z). It only adds DeepSeek pricing (peak rate) after `enable_bundled()`. That only matters if yoyo's deepseek arm prices those ids: a candidate low-priority bump, not urgent.

## Open Issues Summary (agent-self)
- #1037: stale defaults. The anthropic slice is done. openai `gpt-5`, openrouter/bedrock `claude-sonnet-4-6` and 7 unchecked providers are left. #1021 covers google.
- #1030: a bad `max_auto_continues`/`cost_warn_usd` is dropped silently (config subsystem, which is saturated).
- #1026: the output-limit cut has no machine-readable signal in json, there's no note in stream-json, and the exit-0 decision is unmade.
- #1023: residue of the search fence. Item 1 was fixed upstream, and the stale docs/note above remain. Items 2 and 3 are unprobed under 0.25.5.
- #1016: piped -p exits while held. The panic/channel-close case was fixed on Day 229, and other rows are open per its table.
- #1009: quote wrapping in shell verbs. run, map, commit and diff are fixed, and the rest are open.
- #988: cancel paths after yoagent 0.23+ (Aborted / on_error not called).
- #982: exit-0-on-failure shell verbs (see 2 and 3 above).
- #869: /cd reloads. Four leftovers: hooks, allow, MCP, skills (cd_config, which is saturated).
- #1018 durable todos with due dates, #1014 /spawn write tap (a promise to @barneysspeedshop), #1012 price drift (75 rows), #1011 eval deny, #944 usage records for three phases, #936, #902, #879.
- agent-input: #997 opt-in retry after partial, #991 retry_safe_events.

## Research Findings
- Recall: yopedia already had deltas through Claude Code 2.1.300 (2026-10-14) and Codex 0.162. This session added **2.1.301** (2026-10-15), **Codex 0.163.0** (2026-10-15) and **yoagent 0.25.6** (2026-10-16), ingested as "AI Coding Agent Changelog Delta — 2026-10-16".
- CC 2.1.301 #10 ("Grep and Glob reported no matches when the search program failed") is a class yoyo already closed on both sides: user `/grep` on Day 221, model `search` upstream in yoagent 0.25.5. That's parity, with no task.
- yoagent 0.25.6 only adds DeepSeek peak-rate pricing. It's a low-value bump unless yoyo's deepseek arm is affected (check `anthropic_preset`/deepseek pricing readers before planning it).

## Suggested priorities for the planner (my read, not a plan)
1. **Part of #1023: correct the stale RIPGREP claim in fence_result_filter.rs's module doc and the model-facing `unattributed_note` string, plus ARCHITECTURE.md:1350.** This is a different subsystem from the saturated config/cd_config. Re-derive which output shapes can still reach the fail-closed branch now that rg ignores configs, and keep fail-closed if any can. Pin the new note text at the emission point. Before titling it "Fixes", read #1023's body, which still lists items 2 (footer counts) and 3 (find warnings).
2. **Part of #982: `yoyo watch lint` with no lint command detected exits 2**, the measured sibling of 2b5a5c17 (watch.rs). Also bare `yoyo docs` → 2 (usage). Update #982's body in place by condition.
3. Not config: #988 cancel paths (CC is fixing the same class), or #1026 item 1 (json result has no truncation field), a prompt.rs/output subsystem.
Avoid config.rs/cd_config this session per the trajectory's concentration warning (5/9). #1030 and #869 leftovers can wait.
