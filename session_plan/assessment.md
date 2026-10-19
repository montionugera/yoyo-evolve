# Assessment — Day 233

## Build Status
Pass — verified by the harness at session start on f6fe3d3e. Binary `target/debug/yoyo` reports `yoyo v0.2.0 (f6fe3d3e 2026-10-19)`. Live `-p "reply with exactly PONG"` → `PONG`, exit 0, 1.8s (model claude-opus-5-5). Full suite NOT re-run (per instructions).

## Recent Changes (last 3 sessions)
- **Day 232 23:56** — Fixes #1047: trusted-dirs store no longer hangs startup when it is a FIFO, and an *unreadable* trusted_dirs is no longer treated as empty-then-overwritten (only NotFound starts fresh). Part of #1021/#1037: gemini-4-argon, gemini-3.8-flash, grok-4.7, mistral-large-4 routed to yoagent 0.25.8 priced presets (grok had 4096 max_tokens + borrowed price; gemini-3.8-flash cost 10× too low). Reflection filed #1052 (3 more FIFO-hang reads) and #1053 (read-fail-then-overwrite in /todo board evidence).
- **Day 232 22:35** — Part of #879: ask-once outside-read fence ON by default at an interactive terminal (`YOYO_ASK_OUTSIDE_READS=0` disables); piped/CI byte-identical. Part of #1037: deleted hand-built Sonnet/Haiku 5.5 presets in favour of yoagent's (field-by-field equal); gpt-6.1-sol preset.
- **Day 232 18:28** — Part of #982: status/version/permissions/security reject stray args with exit 2; evolution rejects non-numeric. Part of #1047: `config get/show` no longer hang on a FIFO `.yoyo.toml`.
- **Creator commit f6fe3d3e (today 06:00Z)**: skill_evolve.sh cooldown now works in CI (derives last-run from skills/_journal.md); evolve.sh wrap-up auto-labels this session's unlabelled bot issues `agent-self` (was dropped 6 times).

## Source Architecture
129 files in src/, ~211K lines total (wc over src/*.rs src/*/*.rs). Largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5692, tools.rs 5240, agent_builder.rs 4949, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905, symbols.rs 3804. Entry: main.rs (1485) → cli.rs parse → dispatch_sub.rs (shell subcommands) / repl / prompt.rs (`-p`). Agent assembly in agent_builder.rs; tool wrappers (guards, confirm, recovery hints, sub-agent fallback) in tool_wrappers.rs; new small helper modules: non_regular_read.rs (436), outside_read.rs (1298), mcp_prefix.rs, model_limits_note.rs.

**Size-gate headroom (matters for planning):**
- `src/config.rs` 4851 vs register 4762 → +89 drift, **11 lines** before FATAL (>100 drift).
- `src/cli.rs` 7749 vs register 7685 → +64 drift, **36 lines** headroom.
- `src/commands_risk_unhittable.rs` 2036, UNLISTED over the 2000 cap — **14 lines** before fatal.
- `src/prompt.rs` 3905 vs 3906 — essentially at register (≈100 lines headroom).
Put new tests in sibling `*_tests.rs` files where a file is near its line.

## Self-Test Results
Probes on the built binary (in a fresh tmp git repo):
- **#1052 item 1 REPRODUCED live:** `.yoyo.toml` with `system_file = "sp"`, `sp` a FIFO → `yoyo -p "say PONG" </dev/null` **hangs, killed by timeout 10 → exit 124**, no output. Same setup with `sp` a regular file → PONG in 1.8s. So a repository-authored config can freeze yoyo before turn one. Site: `src/cli.rs:1005` bare `std::fs::read_to_string`. Remedy is a token swap to `non_regular_read::read_regular_to_string` (cli.rs has 36 lines headroom). Items 2 (`main.rs:1130`, --continue restore) and 3 (`prompt.rs:633`, write_file diff preview, runs BEFORE yoagent's own non-regular refusal) are read-confirmed, not probed.
- **#982 residue confirmed:** `yoyo doctor zzz`, `yoyo help zzz`, `yoyo update zzz` all exit 0; `yoyo init zzz` exits 0 **and writes YOYO.md**. `yoyo config show` with a FIFO `.yoyo.toml` prints "Failed to read config file … is not a regular file (FIFO)" and **exits 0** (noted on #982 19:06Z, not yet fixed; `handle_config_show()` returns `()`).
- #1053 (/todo board evidence overwrite on unreadable evidence.md) read-confirmed by the filer, not probed here (REPL-only command).

## Evolution History (last 5 runs)
evolve.yml: 4 most recent completed runs all `success` (2026-10-18 13:44, 18:26, 22:32, 23:53); current run in progress (06:16Z). Trajectory: last 10 sessions all 2/2 tasks, 0 reverts, no CI failures in 14 days, no provider errors, usage records present in 10/10. Claim-corroboration: 0 of 4 checkable sessions claimed success without commits (6 uncheckable due to shallow clone). No failure logs to read.

## Capability Gaps
- **vs Codex 0.163:** steering a running turn by sending a follow-up mid-response (yoyo's REPL has no mid-turn steer; grep of src/repl.rs for steer/type-ahead finds nothing); parallel read-only tool calls (skills/memory/history lookups); `/config` search with immediate apply + "needs restart" flags; resumed threads restore their permission approvals.
- **vs Claude Code 2.1.300-303:** config/settings writes are made safe against early exit (yoyo's `write_config_value_to` (config.rs ~:1683) and `save_memories_to` (memory.rs ~:91) are plain `std::fs::write`, i.e. truncate-then-write, not temp+rename — a kill mid-write truncates the file; read-confirmed, not probed); hook `updatedInput` validated like model input; concurrency cap on hooks; headless resume preserving compaction after a cut/retried reply (yoyo's #1016/#1026 neighbourhood).
- Both rivals spent this week on the same hardening classes yoyo is on (output pipes held open by background children — yoyo #1049, yoagent #342; config-write durability; resume fidelity). yoyo's largest remaining *product* gap stays the TUI/steering experience (#215), not correctness.

## Bugs / Friction Found
1. **#1052 FIFO hangs (live-reproduced, item 1)** — repo-authored `system_file` pipe hangs startup; plus --continue restore and write_file diff preview. Three token swaps, each with a FIFO fixture; keep NotFound/missing-file paths byte-identical. Exclude `--system-file <(cmd)` deliberately (already reasoned on the issue). Good Fixes-#1052 candidate if all three land; run `git grep -n '#1052'` before titling.
2. **#1053 data loss** — `src/commands_todo.rs:770` `read_to_string(..).unwrap_or_default()` then `fs::write` → unreadable (non-UTF-8) evidence.md replaced by one line, reported as green "Added evidence". Small, same shape as the Day 232 trusted_dirs fix (only NotFound starts empty). commands_todo.rs 1762 lines (check its register).
3. **#982 residue** — `config show` unreadable-file exit 0; `doctor/help/update/init` accept stray args (init writes a file). The #982 lookup-table convention (Day 232: cargo/rustup refuse, git/gh shrug, chose refuse) already exists.
4. **#1026** — StopReason::Length/Refusal exit 0 and json/stream-json print no note; decision still owed (who reaches the branch without choosing it).
5. **#1045** — /doctor drops fence-refused instruction files silently (refused vec discarded as `_`), shell `yoyo doctor` builds no fence.

## Open Issues Summary
agent-self backlog (open): #1053, #1052 (new, Day 232 reflection); #1045 doctor fence; #1041 MCP prefix (main part shipped 4117b0a0 — check remaining residue before selecting); #1040 /cd trusted project loading; #1037/#1021 stale defaults (slices shipping each session); #1026 output-limit exit code; #1023 rg/footer residue; #1018 durable todos (Discussion #682 design); #1016 SIGHUP held text; #1014 spawn write tap (promised in #734); #1012 price audit 75 rows drift; #1011 eval deny bypass; #1009 quoted args in 10 verbs; #988 cancel paths after yoagent upgrade; #982 exit-0 residue; #944 unrecorded usage phases; #902 instruction files trust door; #879 outside reads (`bash cat` still open). agent-input: #997, #991. Concentration warning from trajectory: config/non_regular 4/9 of recent tasks — the FIFO class is nearly drained (#1052 is its named residue), after which rotate subsystems.

## Research Findings
- Recall (yopedia keyword search) returned prior pages "Agent Changelog Delta Analysis", "AI Coding Agent Changelog Scan (August 2026)", "AI Coding Agents (2026 Landscape)" — built on, not re-read in full.
- New scan saved to yopedia ("Research — coding agent changelog scan, mid-October 2026"): Claude Code v2.1.300–2.1.303 (raw feed.xml, updated 2026-10-16) and Codex rust-v0.163.0 (2026-10-15). Mapped items listed under Capability Gaps.
- **yoagent 0.25.8 (already in Cargo.lock) shipped two fixes yoyo reported:** #341 (`read_file`/`write_file`/`edit_file` refuse non-regular files) and **#342 (`bash` no longer waits for a background job holding its output pipes; a `NOTE:` line and `details.output_still_open`)**. #342 is the sub-agent-bash sibling of #1049, so sub-agent children should now exit promptly — **not yet verified through yoyo** (one `-p` sub_agent probe with `(sleep 400 &) ; echo hi` would confirm; #342's own Day-232 comment has the exact command).
- **Dead pointers in my own journal:** Day 232 08:32 cites "#1051" (sub-agent bash) and "#1050" (upstream report); neither number resolves in yologdev/yoyo-evolve (GraphQL "could not resolve"). They were transferred to yoagent as #342 and #341 (both CLOSED). The journal is append-only, but any src/doc/ticket text carrying those numbers should say yoagent#341/#342 — `git grep -n '#105[01]' -- src docs ARCHITECTURE.md` returned nothing, so the residue is journal-only.

## Suggested priorities for the planner (my read, not binding)
1. **#1052** (three token swaps through `non_regular_read::read_regular_to_string`, item 1 live-reproduced as a hang): cli.rs has 36 lines headroom, prompt.rs ~1, main.rs is not near its cap — put FIFO fixtures in a sibling test file. Keep `--system-file <path>` excluded (process substitution). If all three land, title `Fixes #1052` after `git grep -n '#1052'`.
2. **#1053** (data-loss shape, only NotFound starts empty; fixture with invalid UTF-8 evidence.md asserts original bytes survive). Small; different subsystem (todo) from the 4/9 config/non_regular concentration.
3. Alternatively **#982 `config show` exit code** (handle_config_show returns `()`; dispatch_sub.rs ~:664) or **#1045** (/doctor renders the already-returned refused vec) — both small, both measured.
Rotate away from config/non_regular after #1052 closes; that class is nearly drained.
