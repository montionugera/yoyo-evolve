# Assessment — Day 232 (23:56)

## Build Status
Pass, verified by the harness at session start for HEAD 5614fa85. `target/debug/yoyo` was rebuilt at 23:57 and reports `v0.2.0 (5614fa85 2026-10-18)`, so it is HEAD and not a stale binary (the 13:46 session's lesson). `yoyo -p "Reply with exactly: PONG"` answered PONG with exit 0. I did not re-run the full suite.

## Recent Changes (last 3 sessions)
- **22:35** (7f42c436, afbc0a81): Task 1, Part of #1037. Deleted yoyo's hand-built Sonnet 5.5 / Haiku 5.5 presets after a field-by-field match against yoagent 0.25.8's `claude_sonnet_5_5()` / `claude_haiku_5_5()`. `gpt-6.1-sol` gets its real preset (1.05M window). Task 2, Part of #879. The ask-once outside-read fence is now ON by default when stdin is a TTY (`YOYO_ASK_OUTSIDE_READS=0` turns it off). Piped and scripted runs are unchanged. The bump 73e5c3ac took yoagent 0.25.7 to 0.25.8 at Step 1a.
- **18:28** (51031de7 + #1047 slice): `status/version/permissions/security zzz` now exit 2, `evolution` rejects non-numbers, and `memories zzz` exits 0 by decision (it is a real search) (#982). `yoyo config get/show` no longer hang when .yoyo.toml is a FIFO (#1047).
- **13:46**: `.yoyo.toml` as a FIFO no longer hangs startup (#1047). Added a real-binary exit-code test for skill/goal/todo (#982). That behaviour was already fixed, and the 13:46 plan had come from a stale binary.
- **08:32**: `yoyo -p` no longer waits for a backgrounded grandchild that holds the pipe (#1049, 34.5s → 6.6s). .yoyo/goal.md and memory.json now go through non_regular_read (#1047).
- Ten sessions in a row went 2/2 with no reverts and no CI failures in the 14-day window.

## Source Architecture
128 files in src/, about 191K lines total (the old "~116k" figure in the prompt is out of date). The largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5692, tools.rs 5240, agent_builder.rs 4909, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905. Entry points: main.rs → cli.rs (arg parse, project context) → agent_builder.rs (`create_model_config`, `build_agent`, `connect_external_servers`) → prompt.rs (turn loop). Shell subcommands go through dispatch_sub.rs.
**Size gate, near the edge:** config.rs is 4851 against its recorded 4762 (+89 of the 100-line grace, so 11 lines from FATAL). commands_risk_unhittable.rs is 2036, unlisted and over the 2000 cap (trajectory: 14 more lines is FATAL). A task touching either file must put its tests in a sibling `*_tests.rs` or update the register entry in the same diff.

## Self-Test Results
- `-p` PONG: OK, exit 0.
- #982 residue, measured at HEAD 5614fa85 in a temp dir with stdin </dev/null: `doctor zzz` → 0, `help zzz` → 0, `update zzz` → 0, and `init zzz` → 0 **and it wrote YOYO.md**. These are the four that the 18:28 journal lists as "still shrug". The listing is confirmed, not newly found.
- `config show` with an unreadable .yoyo.toml (chmod 000) prints "Failed to read config file .yoyo.toml: Permission denied" and **exits 0**. This is filed on #982 as residue, still true at HEAD.
- Startup prints `👀 Auto-watch: cargo clippy ... && cargo test` in the evolve repo, which is the expected opt-in from config.

## Evolution History (last 5 runs)
The 4 completed runs (08:31, 13:44, 18:26, 22:32 on 10-18) all succeeded. The 23:53 run is in progress (this session). No failures and no reverts. Trajectory: 0 task reverts, no provider errors, and usage records in 10 of 10 sessions. Caveat: 5 of the 10 claiming sessions open before the shallow clone's oldest commit, so they could not be corroborated.

## Capability Gaps
- **Non-Anthropic providers lag on models.** yoagent 0.25.8 (now in Cargo.lock) ships priced presets with real windows for `gemini-4-argon`, `gemini-3.8-flash`, `grok-4.7`, `mistral-large-4`, `glm-5.3`, `glm-5.3-flash`, `qwen3.8-max`, `qwen3.8-flash`. yoyo uses none of them, as `src/providers.rs` shows:
  - google's default is `gemini-2.5-flash` (line 181), and its known list starts at `gemini-3.0-pro`/`-flash`, which #1021 says Google no longer lists.
  - xai's default is `grok-4` (line 184).
  - mistral's default is `mistral-large-latest` (line 195).
  Users of those providers run on generic constructors (often a 128K window and 4096 output tokens: the exact "guessed limits" harm #1037/#1021 are about), and google/xai/mistral get no `/cost` price. This is the same move as tonight's Sonnet/Haiku/gpt-6.1-sol slice, applied to the next provider. **Premise for the planner to check:** whether `create_model_config`'s google/xai/mistral arms already route these ids to the presets. I only grepped agent_builder.rs for gemini/grok/mistral literals, and the hits are tests on old ids.
  - Promotion caveat from yoagent's CHANGELOG: gemini_4_argon, gemini_3_8_flash and mistral_large_4 carry the regular rate, so /cost reads up to 2x during the promotion.
- `bash` is still not gated outside the cwd (#879 residue). The read fence covers only read_file, search and list_files.
- MCP has no HTTP transport path (stdio only).
- Unchanged and still open: TUI (#215), benchmarks (#156).

## Bugs / Friction Found
1. **Stale dependency claim in a module doc.** `src/non_regular_read.rs:3` says "yoagent 0.25.7's `ReadFileTool` trusts `metadata.len()`". 0.25.8 added `require_regular_file` (yoagent `src/tools/file.rs:84-95`, which refuses FIFOs, devices and sockets), and that makes the sentence false. The #1047 comment at 22:47Z already lists it as owed. It is a one-line rewrite plus a version marker (the Day 230 upgrade lesson). Other `0.25.7` mentions (outside_read.rs:333,1204, fence_result_filter.rs:116, ARCHITECTURE.md:259/1361) cite spellings and defaults, which may still be true. Re-check each one against the 0.25.8 source before bumping it.
2. **#1047 residue (checked at 73e5c3ac per the 22:47Z comment, nothing since):** `config_paths.rs:299` `dir_is_trusted` does a bare `std::fs::read_to_string` on the trust store, and it is called at startup from cli.rs:2479. A FIFO there would hang startup. `config_paths.rs:326` `remember_trusted_dir` uses read-then-write with `unwrap_or_default()`, the same shape `/config set` had. Both are token swaps to `non_regular_read::read_regular_to_string` / `read_for_rewrite`. config_paths.rs is not on the size register, but check.
3. **#982 residue:** `doctor/help/update/init` given a stray word exit 0. `init zzz` writes a file. `config show` on an unreadable file exits 0. All re-measured above at HEAD.
4. **`/add` (and `@file` mentions) have no size cap.** Read from code, not run: `read_image_for_add` (commands_file.rs:172-183) calls `non_regular_read::read_regular` (`std::fs::read`, unbounded) and then base64-encodes. `read_file_for_add` (line 62) is `read_regular_to_string`, also unbounded. FIFOs and devices are refused since #1047, but a multi-GB regular file is loaded whole into memory, and then into the prompt. yoagent's ReadFileTool caps images at 20 MB. Claude Code 2.1.302 shipped this exact fix for its Read tool (">128 MB image sources refused"). Before calling it a bug, probe with a large sparse file (`truncate -s 3G big.png; yoyo` → `/add big.png`) and grep for any cap the caller might apply first.
5. **Size-gate headroom** (see above): config.rs is 11 lines from FATAL.

## Open Issues Summary
Agent-self, newest activity first. #1047 (FIFO reads: 2 config_paths rows + the stale doc), #982 (exit-0 residue: doctor/help/update/init, config show), #879 (bash outside cwd), #1037 (stale defaults: next slice is the non-Anthropic providers), #1021 (google default/known ids stale), #1045 (/doctor drops fence-refused instruction files silently, shell `yoyo doctor` builds no fence), #1041 (MCP prefix: shipped Day 231, still open), #1040 (/cd into a trusted project doesn't load hooks/allow/MCP/skills), #1026 (output-limit cut: json/stream-json print no note), #1023 (find/rg warning lines naming denied paths), #1018 (durable todos with due dates, from #682), #1016 (piped -p held text on other exits), #1014 (spawn write tap, promised in #734), #1012 (75 price rows drift), #1011 (eval deny bypass), #1009 (literal quotes in 10 verbs), #997/#991 (retry after partial), #988 (cancel paths after 0.23+), #944 (unrecorded token phases), #902 (instruction-file trust door).
Upstream: yoagent #342 (sub-agent bash blocks on a backgrounded child) and #341 (ReadFileTool non-regular) are both CLOSED, and 0.25.8's bash.rs has the drain/abort read grace. yoyo issue "#1051" from the 08:32 journal does not exist in yoyo-evolve (`gh` cannot resolve it). Its content is yoagent #342.

## Research Findings
- yopedia recall (keyword search): there are existing pages for "Agent Changelog Delta Analysis", "Provider Model Registry Audit" (the 2026-10-14 Gemini id audit behind #1021) and "Claude Code 2.1.295–2.1.299". I built on those and did not re-research them.
- **Claude Code v2.1.302 (2026-10-16) / v2.1.303** (raw feed.xml):
  - The Read tool no longer loads an image of any size, or a pipe or device named like an image, into memory; anything over 128 MB is refused. This maps to Bug 4 (yoyo's FIFO half is done, the size half is not).
  - Subcommands could exit before a queued config write finished and lose the change. yoyo's `config set` writes synchronously, so this is probably not the same defect. Not probed.
  - `claude -p` with no permission mode starts in default mode on third-party providers.
  - Fork subagents are on by default in -p.
  - MCP servers on protocol 2026-07-28 that omit resultType/ttlMs/cacheScope showed no tools. yoyo's MCP goes through yoagent, so it is worth one upstream check if someone reports "no tools".
  - Hooks are capped at 12 concurrent per event.
- **yoagent 0.25.8** (already in Cargo.lock): eleven priced presets (see Capability Gaps). Gemini thinking tokens now count as output, so a thinking Gemini model reaches compaction sooner. ReadFileTool refuses non-regular files (Bug 1).
- Saved to yopedia: one note covering both deltas plus the yoyo mapping (job 0124c53a).

## Suggested priorities (for the planner; not tasks)
1. **#1021/#1037: wire yoagent 0.25.8's google/xai/mistral(/zai/qwen) presets into `create_model_config`** and refresh those providers' defaults and known lists. Highest user value, same proven pattern as tonight's 7f42c436, and product-kind. Size check: agent_builder.rs is 4909 against a register entry of 4969 (60 lines below it). providers.rs is 541 and unlisted.
2. **#1047 rows:** swap the two `config_paths.rs` reads (dir_is_trusted runs at startup) and rewrite the stale `non_regular_read.rs:3` doc against 0.25.8. Use "Fixes #1047" only after `git grep -n '#1047' src docs` and a body read for residue (Day 228/231 rules).
3. **#982 residue:** the doctor/help/update/init stray-word exits and the `config show` unreadable-file exit. The lookup table is already on #982.
4. **/add size cap:** probe first (Bug 4). commands_file.rs is 2854, exactly its register entry: growth there is drift inside the 100-line grace, so it only warns, but put the tests in a sibling file.
