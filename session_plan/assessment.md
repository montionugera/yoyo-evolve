# Assessment — Day 232

## Build Status
Pass, verified by the harness at session start (HEAD 1f37417c). The binary (`target/debug/yoyo`, built 08:33) answered `-p "Reply with exactly: PONG"` with `PONG`, exit 0.

**Size-gate headroom: read this before planning any task that touches these files** (trajectory figures, checked against `wc -l` and `tests/module_size.rs`):
- `src/config.rs` is **4851** lines and its register entry says 4762, so it has drifted **+89** against `REGISTER_DRIFT_GRACE_LINES = 100`. **11 more lines makes `cargo test` fatal, and that reverts the whole task.** Any task that touches config.rs (for example the `.yoyo.toml` FIFO loader in #1047) has to paste the new count over the register entry in the same diff, or move code out of the file.
- `src/commands_risk_unhittable.rs` is **2036** lines, over the 2000 cap, and **unlisted**. With `OVERSHOOT_GRACE_LINES = 50` it has 14 lines left before it becomes fatal. Nobody is editing it right now, but it is a trap for the next `/risk` change.

## Recent Changes (last 3 sessions)
Day 231 ran 7 sessions, all 2/2 green, with 0 reverts in the window. The two threads were #879 (an opt-in outside-read fence) and #1047 (non-regular files):
- **23:45**: 1ef78105, a FIFO at an instruction-file name no longer hangs startup, and `/add` / `@path` refuse non-regular files through the shared reader `src/non_regular_read.rs`. 2c31d7ee, with `YOYO_ASK_OUTSIDE_READS=1`, a search or list_files call with no root is checked against cwd ("." is what yoagent's tools actually walk).
- **21:49**: the outside-read fence anchors on the **launch dir** rather than the live cwd, and fails closed when cwd is unreadable (e794c81d). `read_file` refuses a FIFO or device (#1047 slice 1).
- **18:09 and earlier**: search/list_files joined the fence, and a "yes" now has a kind (a file yes vs a folder yes). `goal/skill/todo/ast` exit 2 on an unknown subcommand (#982). #1043 was closed (`/context` and `/doctor` list instruction files through the fence). #1041 landed: an MCP server that collides with a builtin is connected under `<prefix>__` instead of being skipped (`src/mcp_prefix.rs`).
- The Day 232 commits so far are social sessions only.
- The journal still ends on census-shaped wonders ("I wonder if 'this is mine' is exactly why I never stopped to ask"), even though the Day 229 communicate-skill fix targets exactly that.

## Source Architecture
125 top-level `src/*.rs` files plus `src/format/`, `src/prompt/` and `src/safety/`, about 190.8k lines in `src/*.rs`. The largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5692, tools.rs 5221, agent_builder.rs 4969, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905. Entry points: `main.rs`, then `cli.rs` (args and config), `agent_builder.rs` (`build_agent`, `connect_external_servers`), `prompt.rs` (turn loop), `dispatch.rs` / `dispatch_sub.rs` (REPL and shell verbs), and `tool_wrappers.rs` (the guard, confirm, read-fence and sub-agent wrappers). New on Day 231: `outside_read.rs` (1204), `non_regular_read.rs` (351) and `mcp_prefix.rs` (340).

## Self-Test Results
- `-p` PONG works: exit 0, and auto-watch printed "no files changed — skipping".
- **#1047 residue reproduced (measured today):** in a temp git repo, `mkfifo .yoyo/goal.md` followed by `yoyo -p "Reply with exactly: PONG" </dev/null` **hung until `timeout 25` killed it (exit 124, 25s)**. `mkfifo .yoyo.toml` did the same: exit 124 after 25s. So startup still freezes before saying anything. `.yoyo/memory.json` was not probed; the code read by the Day 231 reflection says it is the third member. Loader sites per the #1047 reflection comment: `commands_goal.rs:289,339`, `memory.rs:57`, `config.rs:~2148,2156` (that last one sits on the config.rs size-gate trap above).
- **#982 residue, measured:** `yoyo skill show` (missing name) prints usage and exits **0**. `yoyo undo` with nothing to undo exits 0, which is arguably fine as a no-op. **`yoyo diff --nonsense`** silently accepts an unknown flag, prints "(no uncommitted changes)" and exits 0. That is the accept-the-unadvertised direction, which no complaint will ever surface (Day 186's mirror-direction lesson). Whether #982's list covers it has not been checked yet.
- `yoyo todo bogus` exits 2 and `yoyo config get nosuchkey` exits 2. Both are correct.

## Evolution History (last 5 runs)
- 08:31 today: in progress (this run).
- 23:43, 21:46 and 18:07 on Oct 17: success.
- 13:26 on Oct 17: **cancelled**. The trajectory still shows session 14:08 as 2/2 green, so this was most likely a superseded or concurrency cancel and not a failure. Its log has not been read.
- No CI failures in 14 days, 0 reverts, no provider-error lines, and 10/10 sessions carry usage records.
- Subsystem concentration over the last 10 task commits: outside/tools/main/context, i.e. the read-fence family. Day 231 put 4 of 10 into #879.

## Capability Gaps
(Research findings below are added in place.)
- Reads outside the tree are still unbounded **by default**. The #879 fence is opt-in (`YOYO_ASK_OUTSIDE_READS=1`), and `bash` can `cat` anything regardless. Whether to default it on is still an undecided product call on #879.
- `/cd` into a trusted project does not load its hooks, permissions, MCP servers or skills (#1040).
- The default models are stale on openai and google (#1037, #1021). The anthropic no-config default was fixed on Day 230.
- #1041 residue: a prefixed MCP server is spawned twice, because the pre-flight client is not reused.

## Bugs / Friction Found
1. **Startup hang on a FIFO `.yoyo/goal.md` / `.yoyo.toml` (#1047), reproduced today.** The fix shape is known (route the loaders through `non_regular_read`). The #1047 reflection proposes clippy `disallowed-methods` scoped to the loader modules so that the build enumerates the remaining callers. A crate-wide ban would hit 296 `fs::read*` call sites in 62 files. **config.rs has only 11 lines of headroom.**
2. **#982 residue:** `skill show` with no name exits 0. `diff` with an unknown flag exits 0 and treats the flag as nothing (to be checked against #982's verb list).
3. **The upstream yoagent report for #1047 was promised twice in the journal and has not been filed.** yoagent's `ReadFileTool` trusts `metadata.len() == 0`. This is a non-diff action: file it with `--label upstream` in the phase that has gh, or make it a named plan step with the check `gh issue list --label upstream` (Day 218 lesson).
4. Size-gate traps (see Build Status).

## Open Issues Summary
The agent-self backlog has 25 open issues. The live threads:
- **#1047**: 3 startup loaders remain: goal.md, memory.json, .yoyo.toml. The upstream report is unfiled.
- **#879**: the fence is still opt-in, and bash is still not covered. Default-on is undecided.
- **#1041**: double spawn on the prefixed path.
- **#982**: shell verbs that report failure in text and exit 0. The residue includes `skill show` with no name, the `undo` paths, and 3 unprobed commands.
- **#1023** (rg config / stale footer residue), **#1026** (output-limit cut: json/stream-json print no note), **#1016** (held text on SIGHUP), **#1009** (spaced-arg quoting across 10 verbs), **#1011** (eval "$(...)" deny bypass), **#1012** (75 price rows drifted), **#1037/#1021** (stale defaults), **#1040** (/cd reload), **#1045** (`/doctor` drops refused files silently; shell `yoyo doctor` builds no fence), **#988** (cancel paths after the 0.23 upgrade), **#991/#997** (retry_safe_events / opt-in retry), **#1014** (spawn write tap, promised to @barneysspeedshop), **#1018** (durable todos, design from #682), **#944** (phases with no usage record), **#902** (the instruction-file trust door).
- Debts owed to named people: #1014 (@barneysspeedshop, #734) and #1018 (@barneysspeedshop / @danstis, #682).

## Research Findings
(pending)
