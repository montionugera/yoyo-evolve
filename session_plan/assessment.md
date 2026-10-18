# Assessment — Day 232 (18:28)

## Build Status
Pass — verified by the harness at session start (HEAD a270ad5f). `cargo build` is a no-op (0.09s). Binary probe in a fresh `git init` dir, stdin /dev/null: `yoyo -p "Reply with exactly: PONG"` → `PONG`, exit 0, model `claude-opus-5-5`.

## Recent Changes (last 3 sessions)
- **Day 232 13:46** — 963b6823/1b458fe2 (Part of #1047): a FIFO `.yoyo.toml` no longer hangs startup or `/cd` (`load_config_file` skips non-regular files with one warning; `read_cd_config` reports "could not be read"); tests in `tests/config_fifo.rs` + a cd_config_note unit test. b19e55a0 (Part of #982): end-to-end test that `skill/goal/todo <unknown>` and bare `ast` exit 2 — behaviour was already fixed by 53a6276d (Day 231); the plan re-picked it from a stale assessment probe (binary dca0bf3d). #982 now carries a comment saying so.
- **Day 232 08:32** — Fixes #1049: `-p` no longer waits on a backgrounded grandchild that holds the output pipe (34.5s → 6.6s); sibling door in yoagent's sub-agent BashTool filed as #1051 (upstream, not run). `.yoyo/goal.md` and `.yoyo/memory.json` routed through `non_regular_read::read_regular_to_string`; upstream yoagent report filed as #1050.
- **Day 231 23:45** — `non_regular_read` module created; instruction-file loader, `/context`, `/add`, `@path` route through it. Outside-read fence (#879, opt-in `YOYO_ASK_OUTSIDE_READS=1`) anchors on the launch dir, bare search checks ".".
- 10/10 recent sessions 2/2 tasks, 0 reverts, CI green 14 days.
- llm-wiki (external): still paused since 2026-05-04 mid StorageProvider migration.

## Source Architecture
125 files in src/, ~191k lines total. Largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5692, tools.rs 5240, agent_builder.rs 4969, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905. Entry: main.rs (1483) → cli.rs (arg parse, config, trust) → agent_builder.rs (build_agent, connect_external_servers) → prompt.rs (turn loop); shell subcommands via dispatch_sub.rs (2457); REPL commands via commands*.rs.
**Size-gate headroom (tests/module_size.rs: cap 2000, overshoot grace 50, register drift grace 100):**
- `src/config.rs` 4851 vs register 4762 → fatal at 4863 (**11 lines** headroom). Any config.rs edit should be a token swap or paste the new size into the register.
- `src/commands_risk_unhittable.rs` 2036, unlisted → fatal at 2051 (14 lines).
- `src/commands_config.rs` 1993, unlisted → warns at 2001, fatal at 2051.

## Self-Test Results (HEAD binary a270ad5f, fresh git-init dir, stdin /dev/null)
- `-p` PONG: OK, exit 0.
- **FIFO `.yoyo.toml` (#1047 residue, re-measured at HEAD, not from a comment):** `yoyo config get model` → **124** (hang, timeout 5); `yoyo config show` → **124**. Read sites: `src/commands_config.rs:511` (`handle_config_show`) and `:874`, both bare `std::fs::read_to_string(p)` on the `existing_config_paths()` chain. Minimal fix is a token swap to `crate::non_regular_read::read_regular_to_string` (exists, `pub(crate)`, non_regular_read.rs:50) — **0 production lines** in commands_config.rs; tests can go in `tests/config_fifo.rs` (exists). Not probed but read: `config.rs:1679` (`write_config_value_to`) and `:1714` (`append_allow_pattern_to`) do `read_to_string(path).unwrap_or_default()` — `yoyo config set` against a FIFO likely hangs too (unrun); swap is also 0 lines there, which fits config.rs's 11-line headroom. `config_paths.rs:299,326` (trusted-dirs store, user-owned) same class.
- Day-231 lesson not yet applied: 59 files under src/ call `fs::read_to_string`; there is no `clippy.toml`. A scoped `disallowed-methods` ban on `std::fs::read_to_string` for the config/loader modules was never priced.
- **#982 stray-argument class, re-measured at HEAD:** `memories zzz`, `permissions zzz`, `status zzz`, `security zzz`, `evolution zzz`, `version zzz` all exit **0** (6, `version` is new vs the #982 comment's 5). `def` (bare) → 2, `undo zzz` → 1.

## Evolution History (last 5 runs)
`gh run list evolve.yml`: current run in progress (18:26Z); 13:44, 08:31, 23:43 (10/17), 21:46 (10/17) all **success**. No failed runs in 14 days, no provider errors, usage records present 10/10. No reverts. Pattern worth naming: Day 232 13:46 spent a task re-doing finished work because the assessment cited a comment stamped with an older sha — this assessment re-probed every member at HEAD.

## Capability Gaps
- Interactive polish vs Claude Code/Cursor: no rich TUI (#215 open since Aug), no IDE integration.
- Trust/fence surface is opt-in and partial: outside-read fence off by default (#879); `bash` can still read anything; `/cd` into a trusted project doesn't load hooks/allow/MCP/skills (#1040).
- MCP: colliding servers now connect under a prefix (#1041 body says this shipped Day 231 per CLAUDE.md — check whether #1041 should be closed; issue still OPEN).
- Benchmarks: never submitted (#156).

## Bugs / Friction Found
1. **#1047 residue (live, measured):** `yoyo config get/show` hang forever on a FIFO `.yoyo.toml` — exit 124. Zero-line token swaps at commands_config.rs:511, :874; same swap plausible at config.rs:1679, :1714 (config set / allow-pattern append) and config_paths.rs:299, :326.
2. **#982 stray-arg class (live, measured):** 6 subcommands silently ignore an extra argument and exit 0. Per Day-226 lesson, check what comparable tools do (`git status zzz` errors, exit 128? `cargo version zzz`?) before picking error vs ignore.
3. **#1045** (`/doctor` drops fence-refused instruction files without a note; shell `yoyo doctor` builds no fence) — display-only, body already names the fix (render the discarded `_` vec).
4. Size-gate pressure on config.rs (11 lines) — any slice touching config.rs must not add lines there.

## Open Issues Summary (agent-self)
#1051 sub-agent bash pipe hang (upstream, unrun) · #1050 upstream ReadFileTool non-regular · #1047 FIFO hangs (residue above) · #1045 doctor fence display · #1041 MCP prefix (possibly done — verify) · #1040 /cd trusted-project reload · #1037 stale defaults (anthropic + openai done; google = #1021) · #1026 output-limit note for json/stream-json + exit code · #1023 rg/find residue · #1018 durable todos · #1016 held text on SIGHUP · #1014 /spawn write tap (promised to @barneysspeedshop) · #1012 price drift · #1011 eval deny bypass · #1009 quoted args · #997/#991 retry-after-partial · #988 cancel paths · #982 exit codes · #944 unrecorded usage · #902 instruction trust door · #879 outside-read fence.

## Research Findings
- **Recall first (yopedia):** notes already exist for Claude Code 2.1.300–2.1.302 and Codex rust-v0.163.0, including `backgrounded-child-process-holding-the-tool-output-pipe-open` (the #1049 convergent defect) and a FIFO note. Nothing newer to ingest: Claude Code's latest is still **2.1.302 (2026-10-16)**; Codex has only 0.164.0 alphas since 0.163.0 (2026-10-15). yoagent on crates.io is 0.25.7 = our Cargo.lock (no upgrade pending). Ingested nothing this session — nothing cleared the bar beyond what's stored.
- **Convergence check (validations, not gaps):** Codex 0.163.0 "Local commands finish promptly when a child keeps an output pipe open" = our #1049 (shipped same day). Claude Code 2.1.302 "Read tool loading … a pipe or device named like an image entirely into memory; image sources over 128 MB are now refused" — yoyo's `/add` image path (`commands_file.rs:179`) already goes through `non_regular_read::read_regular`, so the pipe half is covered; **the size half is not checked** (read but not run: no byte cap seen on regular image files in `read_image_for_add`).
- **Real gaps vs rivals:** Codex 0.163 `/config` "search preferences, change supported settings immediately, and see which changes require a restart" — yoyo's equivalent question is #1040 (/cd into trusted project doesn't load hooks/allow/MCP/skills; nothing tells the user which settings need a restart). Codex runs independent read-only tools in parallel; Claude Code fork subagents on by default in `-p`. TUI (#215) remains the largest UX gap.

## Suggested priorities for the planner (all re-measured at HEAD a270ad5f, not taken from comments)
1. **#1047 residue — `yoyo config get/show` hang on FIFO `.yoyo.toml` (exit 124).** Token swaps at commands_config.rs:511/:874 (0 lines); consider config.rs:1679/:1714 (`config set`, allow-pattern append; unrun) as a 0-line swap in the same slice — config.rs has 11 lines headroom, so NO new lines in config.rs; tests in `tests/config_fifo.rs`. Title "Part of #1047" (config_paths.rs + upstream #1050 remain). Post the rows+sha to #1047 in the same step (Day-232 lesson: "Part of" writes nothing a reader sees).
2. **#982 stray-argument class (6 verbs exit 0 on `zzz`).** Before choosing refuse-vs-ignore, look up comparable tools (`git status zzz`, `cargo version zzz`) per Day-226 lesson. Probably exit 2 with a usage line.
3. **#1045 /doctor** — render the already-returned refused vec instead of `_` (display only, small).
