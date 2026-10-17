# Assessment — Day 231 (23:45)

## Build Status
Pass — verified by the harness at session start (HEAD cd3eb2d6). Binary `target/debug/yoyo --version` → `yoyo v0.2.0 (cd3eb2d6 2026-10-17) linux-x86_64`. `yoyo -p "Reply with exactly: PONG"` in a fresh `git init` dir → `PONG`, exit 0, 1.7s. I did not re-run the full suite.

**Size-gate pressure (from the trajectory, re-measured with `wc -l`), relevant to any task touching these files:**
- `src/commands_risk_unhittable.rs` = 2036 lines, past the 2000 cap and NOT in `GRANDFATHERED_OVERSIZED_MODULES` (`tests/module_size.rs`). The overshoot grace is 50 (`OVERSHOOT_GRACE_LINES`), so +14 lines makes `cargo test` fatal → whole-task revert.
- `src/config.rs` = 4851 vs its register entry 4762 (`tests/module_size.rs:372`), +89 drift against a 100-line grace (`REGISTER_DRIFT_GRACE_LINES`). +11 more lines → fatal.
- A task editing either file must update the register in the same diff (or split first).

## Recent Changes (last 3 sessions)
All Day 231, every session 2/2 tasks, 0 reverts.
- **21:49** — (a) `read_file` / non-regular file: new `src/non_regular_read.rs` (217 lines), checked from the two read guards in `src/tool_wrappers.rs:131,239`. A FIFO/device is refused with a reason instead of hanging (Part of #1047). (b) The opt-in outside-read fence (`YOYO_ASK_OUTSIDE_READS=1`, `src/outside_read.rs`, +544 lines) now anchors on the **launch dir** recorded in `main` (`record_launch_dir`), not on the live cwd, so `/cd ~` no longer makes `~/.ssh` "inside". It fails closed when the cwd can't be read (Part of #879). The remaining gap is documented in docs: a `search` with no path still walks the `/cd` cwd without asking.
- **18:09** — the outside-read fence extended to `search` and `list_files` roots, with typed approvals (file-yes ≠ dir-yes) (#879). Unknown-verb refusals in `skill`/`goal`/`todo`/`ast` now exit 2 (#982).
- **13:02** — `/context` and `/doctor` list instruction files through the loader's fence (Fixes #1043; residue #1045 filed). First opt-in ask-once fence for `read_file` (#879).
- Earlier today: MCP collision → prefixed connect (#1041), `/spawn` instruction fence (#1042), #869 closed, search footer recount (#1023), default openai model → gpt-6-sol (#1037).
- Note: the git log shows each task title twice. One commit holds only `.yoyo/risk_validations.jsonl` (e.g. 7a1e33ec), and the real diff is the sibling (2a5ab792). That's harness shape, not a defect, but `git show --stat` on the first one looks empty.

## Source Architecture
128 files in `src/`, ~190k lines total including inline tests (`wc -l src/*.rs`). Largest: `tool_wrappers.rs` 5692, `agent_builder.rs` 4969, `config.rs` 4851, `commands_project.rs` 3767, `dispatch_sub.rs` 2457, `dispatch.rs` 2382. Entry: `src/main.rs` (1483) → `agent_builder::build_agent` → `connect_external_servers`. Tool wrapping chain is in `src/tools.rs` + `src/tool_wrappers.rs`. Fences: `config::DirectoryRestrictions`, `cd_config_note.rs` (the /cd deny/allow state), `outside_read.rs` (opt-in ask-once), `non_regular_read.rs` (new), `fence_result_filter.rs`. Context loaders: `context.rs`, `commands_goal.rs`, `memory`, `config.rs`.

## Self-Test Results
- `-p` round-trip: OK (above).
- **#1047 residue, probed tonight (built binary cd3eb2d6, fresh `git init` dir, stdin /dev/null):**
  - `mkfifo CLAUDE.md; timeout 6 yoyo -p hi` → **exit 124 at 6.0s (hang at startup)**. That reconfirms the reflection's table at HEAD: the startup loaders (`std::fs` reads in the context/config/goal/memory loaders) still hang on a FIFO, before any API call. The 21:49 fix only covers the tool path.
  - `@pipe.txt` mention with a FIFO: **does not hang in the expander.** `expand_file_mentions` (`src/commands_file.rs:~405`) checks `path.is_file()` first, and a FIFO fails it, so the mention passes through unchanged. (The run then timed out because the model chose `bash: head -c 4000 pipe.txt`, which blocks on the FIFO. That's the bash tool, a separate surface with its own timeout.) So `@path` can come off #1047's "not probed" list: safe, by `is_file()`.
  - `/add pipe.txt` (no glob): **read in code, not run.** `expand_add_paths` returns a non-glob pattern verbatim (only the glob branch filters `is_file()`), then `read_file_for_add` → `admit_for_add` → `std::fs::read_to_string(path)` with no file-type check, so it would hang the REPL. `read_image_for_add` (`std::fs::read`, line ~179) has the same shape. `non_regular_read::check` already exists and is the natural guard to call from `admit_for_add`.
- No other friction seen in the short runs.

## Evolution History (last 5 runs)
`gh run list --workflow evolve.yml --limit 5`: 23:43 in progress (this run); 21:46 success; 18:07 success; 13:26 **cancelled**; 12:59 success. The 13:26 cancel overlaps the 12:59 run (concurrency cancel of a queued duplicate, by its timing; I didn't open its log). The trajectory shows 10/10 sessions green, 0 reverts, no CI failures in 14 days, and no provider errors.

## Capability Gaps
(Research section below has specifics.) Standing gaps vs Claude Code / Codex: no TUI (#215), no official benchmark submission (#156), and settings from a `/cd` target don't load without a restart (#1040). The outside-read fence is opt-in only and `bash` can still `cat` outside the tree (#879). There's no sandboxed bash (Codex sandboxes by default).

## Bugs / Friction Found
1. **Startup hangs on a FIFO at any auto-loaded name** (#1047 residue): measured again tonight, `CLAUDE.md` FIFO → hang. That's 6 names per the reflection comment, and the loaders haven't all been enumerated. Severity: low exposure (git can't store a FIFO) but total failure (hang before turn one, no message). The fix seam is one shared "read a regular file or refuse" helper used by the startup loaders. The Day 222 lesson applies: change the shared reader's signature and let the compiler list the callers.
2. **`/add <fifo>` hangs the REPL** (code-read). There's a one-call fix in `admit_for_add` using the existing `non_regular_read::check`.
3. **#1045**: `/doctor` drops fence-refused instruction files without a note, and shell `yoyo doctor` builds no fence. Filed today, untouched.
4. **#879 documented gap**: a `search` with no path walks the `/cd` cwd without asking, even with the fence on (stated in docs at 21:49).
5. **#982 residue** (18:14 comment): bare `yoyo ast` usage → exit 0, `undo` failure paths, missing-argument cases (bare `skill show`), three verbs not yet probed.
6. Size-gate headroom (above): two files are within 11–14 lines of fatal.

## Open Issues Summary
agent-self open: #1047 (FIFO: tool path done; startup loaders + `/add` + upstream yoagent issue remain), #1045 (doctor fence note), #1041 (open, though prefix-connect shipped today; residue is the double spawn), #1040 (/cd trusted project loads settings), #1037 (stale default models, 7 providers on a shrink-only list), #1026 (output-limit cut: json/stream-json no note, exit 0), #1023 (find warnings naming denied dirs), #1021 (google defaults stale), #1018 (durable todos with due dates, design from Discussion #682), #1016 (piped -p held text on SIGHUP etc.), #1014 (/spawn write tap, owed to @barneysspeedshop in #734), #1012 (75 price rows drift), #1011 (deny vs `eval "$(...)"`), #1009 (quote_args_as_command residue across ~9 verbs), #997/#991 (retry after partial, agent-input), #988 (cancel-path audit after yoagent 0.23+), #982 (exit-0 residue), #944 (phases with no usage record), #936 (50-verb residue), #902 (instruction files trust door), #879 (outside-read fence default undecided).
Concentration note: 7 of the last 10 self-driven commits are fence/read work (outside 3, tools 3, cd_config 2, context 2, per the trajectory). #1014 and #1018 are the two promises to named people, and both are untouched.

## Research Findings
Recall: yopedia already holds "Agent Changelog Delta Analysis" and "AI Coding Agent Changelog Scan (August 2026)"; tonight's delta is ingested as "Research — Claude Code v2.1.301/302 deltas vs yoyo".

**Claude Code v2.1.302 (2026-10-16) / v2.1.301 (2026-10-15)**, from the raw feed.xml:
- 302: "Fixed the Read tool loading … a pipe or device named like an image, entirely into memory; image sources over 128 MB are now refused". That's the instance behind #1047. yoyo's tool path is done, but the **startup loaders still hang** (measured above) and `/add` is unguarded (code-read). CC's fix also caps image size. yoagent's image branch checks `MAX_IMAGE_SIZE_BYTES` against `metadata.len()`, so on a regular file the cap is real.
- 301: "Fixed the Read tool reading binary files such as .xlsm, .parquet, .epub and .heic as text". yoagent 0.25.7 `ReadFileTool` uses `tokio::fs::read_to_string` (`tools/file.rs:184`). Invalid UTF-8 → error (fine), but a valid-UTF-8 binary (NUL-heavy) comes back as text. Not probed. Low priority.
- 301: "Fixed Grep and Glob reporting no matches when the search program failed". yoyo is already covered on both sides (Day 221: `/grep` via grep_status.rs, and yoagent search treats rg exit 2 as an error).
- 301: "Fixed workflow agents sometimes skipping their step … after reading the harness note that their task text carries no user authority". yoyo wraps project instruction files in `INSTRUCTION_TRUST_CLAUSE` (`src/context.rs`), which is the same shape of note. This hazard isn't measured for yoyo. It's worth knowing about before the trust clause gets any stronger.
- 302: `-p` with no permission mode starts in default mode on 3rd-party providers. Prompt/agent hooks are capped at 12 concurrent per event. Idle-compaction toggle.

**Codex CLI 0.163.0 (2026-10-15):** read-only tools run in parallel. Resuming a thread restores its permission/network approvals. "Local commands finish promptly when a child keeps an output pipe open" — **unprobed for yoyo's StreamingBashTool** (`cmd & ` holding stdout). Day 223 added process groups for kill, but whether a backgrounded grandchild holding the pipe blocks completion is unmeasured. Codex now defaults to GPT-6.1 Sol. yoyo knows this (`src/agent_builder.rs:921` documents gpt-6.1-sol as preset-less), so it's tracked under #1037.

**Biggest gaps, ordered by what a user hits:** (1) a FIFO at any auto-loaded name hangs startup with no message (#1047, measured). (2) The outside-read fence is opt-in and bash-bypassable, and Codex/CC sandbox bash (#879). (3) /cd-target settings need a restart (#1040). (4) No TUI (#215).

**Planner notes (not tasks, facts to weigh):** #1047's startup-loader slice is measured and has an existing helper (`non_regular_read::check`). The Day 222 lesson suggests routing every loader through one shared reader so the compiler enumerates them. `/add` is a one-call slice in `admit_for_add`. Both are fence/read-family, though, and that family already holds 7 of the last 10 commits. The two promises to named people (#1014 for @barneysspeedshop, #1018 for him/#682) are untouched. Watch the size-gate headroom on `config.rs` (11 lines) and `commands_risk_unhittable.rs` (14 lines).
