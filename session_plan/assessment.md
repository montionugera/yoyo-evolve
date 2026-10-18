# Assessment — Day 232 (22:35)

## Build Status
Pass — verified by the harness at session start for HEAD `73e5c3ac`. That HEAD is the Step 1a bump **yoagent 0.25.7 -> 0.25.8** (Cargo.lock only, build and tests passed). CI on `a061df90` and `9cf91cc1` is green. The debug binary is HEAD's (`yoyo v0.2.0 (73e5c3ac 2026-10-18)`).

**This bump changes behaviour that yoyo's tree still describes the old way.** Upgrade lesson (Day 230: the re-read population is the upstream diff, not the marker list). The changed upstream files are `tools/bash.rs`, `tools/{read,write,edit}_file`, `provider/google.rs`, `provider/prices.json` and `types.rs`. See Bugs #1–#3.

## Recent Changes (last 3 sessions, all 2/2 green)
- **18:28**: `fe78e0da` Part of #1047: `yoyo config get/show` no longer hang on a FIFO `.yoyo.toml`, and `/config set` refuses the FIFO. `51031de7` Part of #982: `status/version/permissions/security` refuse a stray arg with exit 2, `evolution` refuses a non-number, `memories zzz` stays 0 by decision (it's a search).
- **13:46**: Part of #1047: a FIFO `.yoyo.toml` no longer hangs startup. Part of #982: a real-binary test pinning that skill/goal/todo exit nonzero (no behaviour change; the plan had re-picked work done Day 231).
- **08:32**: Fixes #1049: `yoyo -p` exits promptly when a backgrounded grandchild holds the bash output pipe (34.5s -> 6.6s), by **aborting** the reader after 2s. `.yoyo/goal.md` and `.yoyo/memory.json` now go through `non_regular_read`.
- yoagent issues yoyo filed (yoagent#341 read_file FIFO, #342 sub-agent BashTool background-pipe, #332 gpt-6.1-sol preset) are **all CLOSED by 0.25.8**. (The 08:32 journal calls two of them "#1050/#1051". Those numbers don't exist in yoyo-evolve; they are yoagent#341/#342.)

## Source Architecture
125 files in `src/*.rs`, ~191K lines. Largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5692, tools.rs 5240, agent_builder.rs 4969, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905. Entry: `main.rs` -> `cli.rs` (args/config), `dispatch_sub.rs` (shell subcommands), `repl.rs`/`prompt.rs` (turns), `agent_builder.rs` (provider presets, MCP/OpenAPI connect), `tools.rs` (StreamingBashTool, sub-agent tools), `tool_wrappers.rs` (guards/decorators), `non_regular_read.rs` (#1047 reader).

**Size-gate headroom (grace 50 over the recorded value)**:
- config.rs: 4851 vs 4762, **11 lines left**.
- tools.rs: 5240 vs 5207 (+33), **~17 left**.
- agent_builder.rs: 4969 vs 4969 (50 left).
- commands_risk_unhittable.rs: 2036, UNLISTED, **14 left**.

Put tests in sibling `*_tests.rs` files. A deletion-heavy slice (agent_builder hand-built presets) frees room.

## Self-Test Results
- `yoyo -p` with a real bash call works (5.3s round trip).
- **Measured the #1049 trade live.** `(sleep 4; echo late; echo alive > /tmp/alive_marker) & echo started` through `yoyo -p`: the marker was **never written**, because the job died on its first late write to the aborted pipe. The control without `echo late` wrote `/tmp/alive2`. yoagent 0.25.8's own BashTool (#342) handles this case without the trade (Bug #1).
- #982 residue, re-measured on 73e5c3ac in a fresh git-init dir: `doctor zzz` 0, `help zzz` 0, `update zzz` 0, `init zzz` 0 (and it wrote YOYO.md), `health zzz` 1 (no project). `config show` with a FIFO `.yoyo.toml`: stderr `Failed to read config file … (FIFO)`, **exit 0**. That is the open #982 row named on #982 at 19:06Z, not fixed by 51031de7.

## Evolution History (last 5 runs)
evolve.yml: 4 success (17-10 23:43, 18-10 08:31, 13:44, 18:26) plus this run in progress. No failures, reverts or provider errors in the 14-day window. Trajectory: 10/10 sessions 2/2. Concentration is high: non_regular 4/9 and tools 4/9 of the last 9 self-driven commits (the #1047 / #982 sweep).

## Capability Gaps
- **Model coverage is the live product gap, and the creator wrote the spec tonight.** @yuanhao, #1037 at 22:30Z, checked at a061df90:
  - Add `"gpt-6.1-sol" => ModelConfig::gpt_6_1_sol()` to `openai_preset` (agent_builder.rs ~:925) and make it the OpenAI default. Check `GPT_6_OUTPUT_MAXIMUM` (:939).
  - **Delete yoyo's hand-built `claude_sonnet_5_5` (~:861) and `claude_haiku_5_5` (~:898)** in favour of yoagent's presets. Compare the window and output values first.
  - Per-model arms for `gemini-4-argon`, `gemini-3.8-flash`, `grok-4.7`, `mistral-large-4`, `glm-5.3`, `glm-5.3-flash`, falling back to the generic constructors (google ~:1038, xai ~:1057, mistral ~:1095, zai ~:1110).
  - Skip Qwen (no provider door).
  - Check `/cost` against the bundled prices.
  - Review `default_model_for_provider`. Today `providers.rs:180` defaults google to `gemini-2.5-flash`, and the known lists are stale: openai `gpt-5…`, google `gemini-2.5/2.0`, xai `grok-4/3/2` (#1021 too).
  - **Tests that go stale:** `model_limits_note.rs:141-177` and `:279-281` pin "gpt-6.1-sol has no preset". Re-point them at a still-unknown id; don't delete them.
  - "Part of #1037" per slice. Close it when every default has a preset and no hand-built copy is left. Before titling a slice "Fixes", also run the d231 pointer grep for `#1037`.
- Long-standing gaps vs Claude Code/Codex: no HTTP MCP, no TUI (#215), no benchmark submission (#156), `bash` reads outside the cwd ungated (#879 fence is opt-in).

## Bugs / Friction Found
1. **yoyo's StreamingBashTool kills a background job that prints late, and the dependency no longer does.** tools.rs ~:601-613 aborts the reader after 2s (#1049). yoagent 0.25.8 `tools/bash.rs` (#342) instead waits for `bash` itself, reads for `READ_GRACE` (1s), then **keeps reading and discarding in the background**, so the job is not SIGPIPE-killed, and it adds a `NOTE:` line plus `details.output_still_open`. Measured above: under yoyo the job dies. This is the d221 twin rule: read the reference before re-deciding.
   - Constraint: keep #1049's prompt exit (pinned in `src/tools_group_kill_tests.rs:149+`).
   - In `-p`, the job still loses its pipe when yoyo exits. The gain is in the REPL and in long `-p` runs.
   - tools.rs has ~17 lines of headroom, so tests go in the sibling file.
2. **Stale claims falsified by the 0.25.8 bump** (same-diff rewrite per d230):
   - `src/non_regular_read.rs:3` says "yoagent 0.25.7's ReadFileTool trusts metadata.len()…". 0.25.8 now refuses non-regular files in read/write/edit itself (yuanhao on #1047 at 22:22Z: "Bumping yoyo to 0.25.8 should be enough to close this"). yoyo's wrapper (`tool_wrappers.rs:126,234`) is now belt-and-braces for the tools. It is still needed for yoyo's own std::fs loaders.
   - `ARCHITECTURE.md:1530` says "until yoyo is on yoagent ≥ 0.25.2, the sub-agent's raw BashTool still kills only bash". yoyo has been past that version for a while.
   - Upstream behaviour change: `write_file` to `/dev/null`/`/dev/tty` is now refused.
3. **#1047's remaining rows:** `src/config_paths.rs` ~:299 `read_to_string`, and ~:326 `read_to_string(..).unwrap_or_default()` (read-then-write shape). The upstream half is closed. Once these two are swapped and the stale comment is fixed, #1047 can close. Run the pointer grep for `#1047` first: `git grep -n '#1047'` across src/docs.
4. **#982's remaining rows** (measured above): `config show` unreadable exit 0 (`handle_config_show()` returns `()`, call at dispatch_sub.rs:708), plus `doctor/help/update/init zzz`, where init writes a file.
5. Non-diff, owed now: reply to @yuanhao on #1047 and #1037 once slices land. Each fix's ticket comment should name the rows and sha (d232 'Part of' carries no state).

## Open Issues Summary
agent-self backlog (27 open):
- Active sweeps: #1047 (FIFO reads, near closable), #982 (exit-code class, ~5 rows), #1037 (creator spec above), #1021 (google ids), #879 (outside reads, opt-in fence), #1040 (/cd trusted-project loading), #1041 (MCP prefix: shipped Day 231 as "Part of", check residue), #1045 (doctor fence rows), #1023 (find warnings naming denied paths).
- Older open items: #1026 (output-limit json note), #1016 (piped -p SIGHUP), #1012 (75 price drift rows; 0.25.8's prices.json may shrink it, so re-run the audit), #1011, #1009 (quoted args, 10 verbs), #997/#991 (retry after partial), #988 (cancel paths), #944, #902, #1018 (durable todos, @barneysspeedshop design), #1014 (spawn write tap owed to @barneysspeedshop).

## Research Findings
- **yopedia recall:** prior pages include "Agent Changelog Delta Analysis" and "AI Coding Agent Changelog Scan (August 2026)". Nothing on the bash background-pipe policy before tonight. I saved a new note: *non-regular file reads and background-job pipes across agents (2026-10-18)*.
- **Claude Code v2.1.302 (2026-10-16)** fixed its Read tool loading "a pipe or device named like an image entirely into memory". That is the #1047 class, converging with yoagent 0.25.8, which put its check before the image branch. Check yoyo's `/add` image path (`commands_file.rs:179` uses `read_regular`, so it's probably covered).
- Same release: "subcommands exiting before a queued settings/config write finished" (a durable-write-at-exit class) and "workflow scripts refused after a Bash cd" (the anchor-moves class yoyo met in #879).
- **yoagent 0.25.8 is the most actionable "research" this session.** It's a twin reference for Bug #1 (bash background drain), it supplies presets that make #1037's slices mostly wiring plus deletion, and it now counts Gemini thinking tokens as output (google sessions reach compaction and `max_total_tokens` sooner, and their cost is no longer undercounted). The new bundled prices may also shrink #1012's 75 drift rows; re-run the audit before working that ticket.
- **Non-diff action done in this phase:** commented on #1047 at the bump's sha. The upstream half is closed; three yoyo-side rows remain (config_paths.rs:299, :326, and the stale non_regular_read.rs:3 claim).

## Suggested priorities for the planner (not task files)
1. **#1037 slice (creator spec, product).** Use yoagent's `claude_sonnet_5_5`/`claude_haiku_5_5` and delete yoyo's hand-built copies (net negative lines in agent_builder.rs). Add the `gpt-6.1-sol` arm and make it the OpenAI default. Re-point the `model_limits_note.rs` tests at a still-unknown id. Title "Part of #1037".
2. **Bash background drain (product).** Mirror yoagent 0.25.8's #342 policy in StreamingBashTool: keep reading and discarding in the background instead of aborting, keep #1049's prompt exit, and use the live probe above as the before/after. Ledger every difference from the reference (d223). tools.rs has ~17 lines of headroom; tests go in tools_group_kill_tests.rs.
3. **Close-out slices:** the #1047 config_paths rows plus the stale comments (non_regular_read.rs:3, ARCHITECTURE.md:1530). #982 `config show` exit code.
