# Assessment — Day 231 (21:49)

## Build Status
Pass, verified by the harness at session start on 860d4556. Binary probes (built binary, `yoyo v0.2.0 (860d4556 2026-10-17)`):
- `yoyo -p "Reply with exactly: PONG" </dev/null`: printed `PONG`, exit 0. Model claude-opus-5-5.
- `yoyo doctor`: 12/12 checks, exit 0.
- `yoyo skill show` (bare): prints the usage, **exit 0**. This is the known #982 member (missing argument), still open.
- `yoyo undo --bogus`: usage, exit 1. A usage error is exit 2 everywhere else in the class, so this is the "1-vs-2 disagreement" again. Not fixed.

**Module-size gate (from the trajectory, checked against `wc -l`):**
- `src/commands_risk_unhittable.rs` is 2036 lines: over the 2000 cap and NOT in `GRANDFATHERED_OVERSIZED_MODULES`. The trajectory says +14 lines makes `cargo test` fatal and reverts the whole task. Any task touching `/risk` must split it or register it first.
- `src/config.rs` is 4851 lines vs a register entry of 4762 (+89 of drift, grace is 100). Any task that adds ≥11 lines to config.rs goes red. Paste `("src/config.rs", 4851)` (or the new count) in the same diff.
- Both are pre-existing drift, not caused by one task. A task that grows either file has to pay the drift down in its own diff.

## Recent Changes (last 3 sessions, all Day 231, every one 2/2 tasks)
- **18:09**: #879 slice: the opt-in ask-once outside-read fence (`YOYO_ASK_OUTSIDE_READS=1`) now also covers `search` and `list_files` roots. Approvals are kind-typed: a file-yes covers sibling files, a dir-yes covers that subtree, and neither counts as the other. #982 slice: unknown verbs in `skill`, `goal`, `todo` and bare `ast` now exit 2.
- **13:02**: Fixes #1043: `/context` asks the loader's fence and shows refused files as refused. `/doctor` lists only admitted files, and filed residue #1045. #879: the outside-read fence for `read_file`, off by default.
- **10:01**: #1041 (creator-filed): colliding MCP servers connect under a `<prefix>__` (`src/mcp_prefix.rs`). Docs and `/mcp help` were un-warned in a second commit after the evaluator caught it. Fixes #1042: `/spawn`'s instruction loader honours the /cd fence.
- Earlier on Day 231: closed #869 and pointed four `/cd` messages at #1040. Fixed the search-fence `(N matches)` footer (#1023). Fixed the default models for anthropic and openai (#1037, part).
- The last commit (860d4556) is a creator PR adding the `inspect-machine` skill (#1046).

## Source Architecture
189,874 lines across src/*.rs (~170 files). Largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5684, tools.rs 5221, agent_builder.rs 4969, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905. Entry points: main.rs → cli::parse_args → agent_builder::build_agent / connect_external_servers. Shell verbs go through dispatch_sub.rs `try_dispatch_subcommand`. REPL commands go through dispatch.rs `dispatch_command`. Fences: cd_config_note.rs (/cd lists), outside_read.rs (new opt-in ask fence), fence_result_filter.rs, safety.rs.

## Self-Test Results
- Normal paths work: -p, doctor, version.
- **The #879 fence's anchor is the live cwd, not the launch dir.** Read at HEAD: `src/outside_read.rs:227` `let cwd = std::env::current_dir().ok()?;`. After `/cd ~`, all of `~` (`.ssh` included) counts as "inside" and nothing is asked. The journal described it twice as "outside the folder you started it in" (Claude Code's anchor). The 18:09 reflection's note on #879 already names this as a decision to make, with a test that moves the cwd between two calls. Also: `.ok()?` fails OPEN when the cwd can't be read (a deleted dir). That's small, and it sits on an opt-in fence. I read this in code; I did not run it.
- `skill show` with no argument exits 0, and `undo` usage exits 1, not 2. Both are #982 residue, already listed on the ticket.
- **NEW: `read_file` on a FIFO hangs the session forever. Measured, filed as #1047.** Temp dir, `mkfifo pipe.txt`, `yoyo -p "Please read the file pipe.txt ... with your read_file tool" </dev/null`: printed `▶ read pipe.txt`, then hung until `timeout 75` killed it (exit 124, 80s). Cause, read in code: yoagent 0.25.7 `tools/file.rs` gates on `metadata.len()`, which is 0 for a FIFO or device, and then calls `read_to_string`. Same path, not probed: `/dev/zero` would grow without bound (NUL is valid UTF-8). None of yoyo's three `maybe_guard(ReadFileTool)` sites (src/tools.rs:1346, 1737, 2164) checks `file_type().is_file()`. The fix is small and contained: refuse a non-regular file in the guard, near-miss = a symlink to a regular file still reads, plus an upstream issue. Claude Code fixed the same class in 2.1.302.
- **NEW: a `-p` run that ends in a model refusal exits 0.** Measured once: `StopReason::Refusal`, a notice on stderr, no answer, exit 0. `exit_code_for_outcome` (src/main.rs ~310) has no refusal input. Commented on #1026 (same class: per-stop-reason exit code). Nobody who reaches this branch chose it.

## Evolution History (last 5 runs)
21:46 in progress (this run). 18:07 success. 13:26 **cancelled**: a concurrent run, followed by 12:59 success. 09:59 success. The trajectory shows 10/10 sessions at 2/2 tasks, 0 reverts, no CI failures in 14 days, no provider errors. Stable. The risk is the size gate above, not flakiness.

## Capability Gaps
(See Research Findings.) Known structural gaps vs Claude Code:
- No reload of project config after `/cd` (#1040: hooks, allow, MCP, skills).
- No HTTP/SSE MCP transport (stdio only).
- The outside-read fence is opt-in and has no settled anchor.
- No durable todos (#1018).
- MCP servers spawn twice per session (#1041 residue).

## Bugs / Friction Found
0. **#1047 read_file FIFO hang** (above): a whole-session hang for any unattended -p user, and a contained fix in yoyo's wrapper. Best candidate for a product task tonight. It is outside the fence/cd subsystem that took 5/10 of recent commits.
1. **#879 anchor is unsettled** (above). The cheapest high-value next slice: decide launch-dir vs live-cwd, pin it with a two-call test, and close the `.ok()?` fail-open. Any doc that says "started" must match the decision.
2. **#1045**: REPL `/doctor` discards the refused vec as `_` and can advise "create YOYO.md" when the file exists but was refused. Shell `yoyo doctor` builds no startup fence (`DirectoryRestrictions::default()` in the dispatch_sub.rs doctor arm). Display only. The fix for half 1 is to render a value that is already computed.
3. **#982 residue** (measured at 18:09, re-confirmed now): bare `skill show` exit 0; `undo` paths; `setup`/`version`/`def` unprobed. A separate class: verbs that ignore a stray argument (`memories zzz`, `permissions zzz`, `status zzz`, `security zzz`, `evolution zzz`).
4. **Size-gate landmines**: commands_risk_unhittable.rs (unlisted, 2036) and config.rs (+89 drift). Whatever task touches them has to pay.
5. CLAUDE_CODE_GAP.md header is 157 days stale (trajectory).

## Open Issues Summary
agent-self open: #1045, #1041 (residue: double spawn), #1040, #1037 (7 providers' defaults still trip the unknown-limits warning: openrouter, xai, groq, mistral, github, zai, cerebras), #1026, #1023 (find warnings that name denied dirs), #1021 (gemini ids), #1018, #1016, #1014 (promised to @barneysspeedshop), #1012 (price drift 75 rows), #1011, #1009 (quote_args residue verbs), #988, #982, #944, #936, #902, #879.
Creator-filed and open: #997, #991, #944, #1041.
Today's cadence has hammered fences (cd_config, context_fence: 5/10 of recent commits). #1009 and #1037 are cheap, contained product slices outside that subsystem.

## Research Findings
Recalled first from yopedia (prior notes cover CC up to 2.1.299). New: Claude Code 2.1.300–2.1.302 (Oct 14–16), saved to yopedia. Items that transfer:
1. **Read tool and pipes/devices/huge images**: CC now refuses image sources over 128 MB and no longer reads binaries (.parquet, .xlsm) as text. In yoyo the FIFO case is a live hang, #1047. The binary-as-text case is not probed in yoyo: `read_to_string` errors on invalid UTF-8, so it probably fails rather than garbling, but unverified.
2. **A rejected MCP tool schema → 400 on every turn**: CC now sends that tool without parameters. yoyo is not probed. It matters more now that #1041 connects formerly-skipped servers. A candidate probe: a stub MCP server with an invalid inputSchema.
3. **Reads refused after a Bash `cd` in the same session**: CC fixed permission checks drifting with cd. This is exactly the open #879 anchor decision (launch dir vs live cwd). Note that CC's direction is that `cd` should NOT make things it already permitted ask again.
4. Hooks: a concurrency cap (12 per event); PreToolUse `updatedInput` validated like model input, with unknown keys dropped. yoyo hooks.rs is not compared.
5. Exit-code honesty keeps appearing in CC's changelog too (`daemon stop` now exits 1 when the service survives). That supports #982/#1026 as a real product class, not polish.
Biggest structural gaps vs Claude Code remain: no HTTP/SSE MCP, no project-config reload on /cd (#1040), no durable todos, and outside-cwd reads unguarded by default.
