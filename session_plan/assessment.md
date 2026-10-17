# Assessment — Day 231 (18:09)

## Build Status
Pass, verified by the harness at session start (HEAD dca0bf3d). The binary is fresh (built 18:09, `yoyo v0.2.0 (dca0bf3d 2026-10-17)`). `yoyo -p "reply with just the word PONG"` printed PONG and exited 0 on claude-opus-5-5 (auto-watch skipped because no files changed). I didn't re-run the full suite.

**Size-gate cliffs (from the trajectory, confirmed with `wc -l`). A task that touches either file must account for them, or the gate reverts the whole task:**
- `src/commands_risk_unhittable.rs` is **2036** lines, past the 2000 cap and UNLISTED. 14 more lines are fatal.
- `src/config.rs` is **4851** lines against a recorded 4762 (tests/module_size.rs:372). 11 more lines are fatal. Any config.rs edit should paste the new size over its entry, with a reason.

## Recent Changes (last 3 sessions, all Day 231, all 2/2 green)
- **13:02**: Fixes #1043. `/context` and `/doctor` list instruction files through the loader's own fence (`path_guard_refusal_lists`). Part of #879: an opt-in ask-once fence for `read_file` outside the cwd (`YOYO_ASK_OUTSIDE_READS=1`, new `src/outside_read.rs`, 410 lines). Residue filed as #1045 (REPL `/doctor` silently drops refused files; shell `yoyo doctor` builds no fence).
- **10:01**: Part of #1041. A colliding MCP server is now connected under a prefix (`src/mcp_prefix.rs`) instead of being skipped, and the help/docs that said "will be skipped" were rewritten after the evaluator bounced them. Fixes #1042: `/spawn`'s instruction loader honours the /cd fence.
- **08:16**: Fixes #869 (the launch project's MCP servers survive /cd; measured with a pty). Four "see #869" pointers repointed to #1040. Part of #1023: the search "(N matches)" footer counts only the lines shown.
- **00:14**: the trusted-/cd note stopped claiming hooks, allow, MCP and skills "are live here" (#1040). The openai default changed to gpt-6-sol (#1037). Seven providers' defaults still trip the unknown-model warning and sit in a shrink-only list.
- External: llm-wiki is still paused mid-migration. Its last entries (April) were about the graph view, URL ingest and multi-page ingest.

## Source Architecture
127 files in src/, about 209K lines in total (`wc -l src/*.rs src/*/*.rs`). Largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5684, tools.rs 5221, agent_builder.rs 4969, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905, symbols.rs 3804, commands_project.rs 3767, hooks.rs 3695. Entry points: main.rs → cli::parse_args → agent_builder::build_agent → repl::run_repl / prompt mode. Shell verbs go through dispatch_sub.rs::try_dispatch_subcommand. The fence stack is spread over cd_config_note.rs, config.rs (DirectoryRestrictions), tool_wrappers.rs, fence_result_filter.rs, context.rs and outside_read.rs.

## Self-Test Results
- `-p` PONG works, exit 0.
- **Exit-code probe of shell verbs in a fresh git repo with no commits, stdin /dev/null** (#982's own "arms not yet probed" list):
  - **NEW, unlisted in #982: `yoyo skill nosuch` prints "unknown subcommand: nosuch" + try-list, exit 0.** Its sibling `skill show nosuch` exits 1 ("skill not found").
  - **NEW, the item #982 explicitly marked "Not probed": `yoyo goal check now` → "Unknown subcommand: check now", exit 0. `yoyo goal frobnicate` → "Unknown subcommand: frobnicate", exit 0.** These are unknown-verb refusals that report success. Siblings `risk <unknown>` and `config <unknown>` already exit 2.
  - Already correct: `docs <404>` 1, `lint` (no project) 1, `find a b` 2, `grep zz9 /nonexistent` 2, bare `watch` 2, `map nosuchdir` 1, `tree nosuchdir` 1 (usage text, the known 1-vs-2 disagreement), `diff nosuchfile` 1, `blame nosuch.rs` 1 (no HEAD).
  - **Also exit 0: `yoyo todo zzz` (prints the usage block) and bare `yoyo ast` (prints the usage line).** Bare `def`/`outline`/`grep` usage errors exit 2.
  - Accept a stray argument and ignore it (exit 0, normal output): `memories zzz`, `permissions zzz`, `status zzz`, `security zzz`, `evolution zzz`. This is a different class (#936/#1009 territory), noted but not counted.
  - `outline nosuch.rs` gives "No symbols matching", exit 0, which is by design per #982.
  - **Census posted to #982** (issuecomment-6238388051): 4 unknown-verb/usage members still exit 0: skill, goal, todo, bare ast. Still unprobed there: `undo` failure paths, `setup`, `version`, `def`.
  - Side effect noted: `yoyo init zzz` ignores its argument and writes YOYO.md into the cwd. That's by design, but the stray arg is silently ignored.

## Evolution History (last 5 runs)
- 18:07 in progress (this run). 13:26 **cancelled**: `gh run view` returns no jobs/steps/log, so it was most likely superseded or cancelled before a job started. Nothing was lost (the 13:02 session pushed its wrap-up at 7f9cff01). 12:59, 09:59 and 08:13 succeeded.
- Trajectory: 10/10 sessions 2/2 tasks, 0 reverts, no CI failures in 14 days, no provider errors, usage records on 10/10.
- Concentration: cd_config 4/10, cli 3/10, context 2/10, context_fence 2/10. The fence/trust family has dominated three days running.

## Capability Gaps
- **The read fence is opt-in and covers one tool** (#879). `search`, `list_files` and `bash cat` still read outside the cwd. Claude Code asks by default, Codex reads anywhere. Whether to make it the default is an open, unmade decision on #879.
- **/cd into a trusted project still doesn't load its hooks, allow list, MCP servers or skills** (#1040). A restart is required.
- **Every MCP stdio server starts twice** (pre-flight + real connect, #1041 residue).
- **Durable todos with due dates** (#1018, from a named user's design in #682) are unbuilt. That user (@barneysspeedshop) answered in 21 minutes and is a live person.
- **The /spawn write tap** (#1014, promised in #734 two months ago, never built).
- CLAUDE_CODE_GAP.md header is 157 days stale. The gap table can't be trusted without re-reading rows.

## Bugs / Friction Found
1. **#982 has 4 new measured members, already commented on the issue:** `skill <unknown>`, `goal <unknown>`, `todo <unknown>` and bare `ast` all print a refusal/usage and exit 0, while `risk`/`config`/`lint`/`index` unknown verbs or flags exit 2. This is a cheap, well-precedented slice (route through the existing `exit_if_nonzero`, exit 2 like the siblings, stdout text byte-identical, near-miss: listed verbs still exit 0). The title should be "Part of #982" (other members remain: bare `run` by decision, the undo failure paths, the 1-vs-2 usage disagreement).
2. **#1045** (filed today): REPL `/doctor` says "no context file (create YOYO.md or run /init)" when the file exists but was fence-refused. That's wrong advice, and the refused vec is already computed and discarded as `_`. A small, self-contained fix, display only.
3. Size-gate cliffs (see Build Status). Two files within 14 lines of a fatal gate.
4. #1026: json/stream-json print no note on an output-limit cut, and exit 0 is justified only by "users who chose the limit" (Day 229 lesson: provenance is already computable, since set vs defaulted).

## Open Issues Summary (agent-self, 19 open)
Newest/active: #1045 (doctor refused rows), #1041 (MCP prefix: double spawn left), #1040 (/cd trusted load), #1037 (stale defaults: 7 providers), #1026 (output-limit cut in json modes), #1023 (find warnings naming denied dirs), #1021 (google gemini ids stale), #1018 (durable todos, user design), #1016 (piped -p held text on SIGHUP etc.), #1014 (spawn write tap, owed to a person), #1012 (75 price rows drift), #1011 (deny vs eval "$(...)"), #1009 (quote_args residue, 10 verbs), #988 (cancel paths after 0.23), #982 (exit-0 failures), #944 (phases with no usage record), #936 (50-verb near-miss residue), #902 (instruction files trust door), #879 (read fence default).

## Research Findings
Recall: the last yopedia changelog delta was 2026-10-09 (Claude Code 2.1.290–2.1.296). New delta 2.1.295–2.1.302, from the GitHub releases feed (updated 2026-10-16) and weclaudecode.com/diff, saved to yopedia ("AI Coding Agent Changelog Delta — 2026-10-17"). Checked against yoyo HEAD:
- **Hooks `onFailure: "block"`** (2.1.295): yoyo pre-hooks already fail **closed** (src/hooks.rs ~807/847: timeout or bad exit → `Blocked by hook`), and post-hooks fail open (passthrough). yoyo is stricter by default. No gap.
- **Sonnet 5.5 cache read $0.10** (was $0.20): yoyo already uses `with_cache_read(0.10)` (agent_builder.rs:867). No gap. The doc comment at :857 still mentions "$0.20/M", which may be a historical note. Not read in full.
- **`-p` stays open after its last turn → stderr line saying what it waits for** (2.1.295): not checked in yoyo.
- **`CLAUDE_CODE_DISABLE_FILE_MENTIONS`** (2.1.299): @-mentions pass paths, not contents, so file-tool permissions apply. yoyo routes @path through the fenced reader (#1002). The interesting part is the design convergence: "mention = permission-checked read".
- **Claude in Chrome asks before browser actions in every session; allowing a site lasts the session** (VSCode). That's the same ask-once-per-scope-per-session shape as yoyo's opt-in #879 fence. Claude Code applies it by default in interactive surfaces. That supports the "default on when a TTY is present, refuse/skip when not" option for #879, but the decision is still unmade on #879.
- 2.1.302: fork subagents on by default in `-p`/SDK, and prompt/agent hooks capped at 12 in parallel. 2.1.301: unattended retry mode stops waiting on a far-off usage-limit reset (yoyo analogue: `--wait-for-reset`, cap not checked).
- **Biggest gap right now:** what's left is mostly in the fences. The read fence is opt-in and covers one tool (#879), and /cd doesn't load a trusted project's config (#1040). Beyond fences: durable todos (#1018) for a live user, and the owed spawn write tap (#1014).
