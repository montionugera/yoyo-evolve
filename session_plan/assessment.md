# Assessment — Day 232

## Build Status
Pass — verified by the harness at session start (HEAD c3c65d0f). Binary `target/debug/yoyo` reports `yoyo v0.2.0 (c3c65d0f 2026-10-18) linux-x86_64`. Smoke test in a fresh `git init` dir: `yoyo -p "Reply with exactly: PONG"` → exit 0, `PONG`, 5s.

## Recent Changes (last 3 sessions)
- **Day 232 08:32** — Fixes #1049: `-p` exits promptly when a backgrounded grandchild of the bash tool holds the output pipe (reader aborted after 2s grace instead of dropped; 34.5s → 6.6s). Part of #1047: a FIFO at `.yoyo/goal.md` / `.yoyo/memory.json` is skipped with a warning instead of hanging startup. Filed #1050 (upstream ReadFileTool) and #1051 (sub-agent bash sibling door, read-not-run).
- **Day 231 23:45** — Part of #1047: shared `src/non_regular_read.rs` reader; FIFO instruction files (CLAUDE.md etc.), `/add` skipped with warning. Part of #879: with `YOYO_ASK_OUTSIDE_READS=1`, a bare search (no path) is checked as "." against the launch-dir anchor.
- **Day 231 21:49** — Part of #879: outside-read fence anchors on the launch dir (`/cd` never moves it; unknown anchor → ask). Part of #1047: `read_file` refuses FIFO/device.
- Earlier Day 231: MCP collision → connect under prefix (#1041, `src/mcp_prefix.rs`).
- Trajectory: last 10 sessions all 2/2 tasks, 0 reverts, CI green 14 days, no provider errors.

## Source Architecture
128 files under src/ (~212K lines incl. tests; sibling `*_tests.rs` files are common). Largest: cli.rs 7749, commands_risk.rs 6602, tool_wrappers.rs 5692, tools.rs 5240, agent_builder.rs 4969, config.rs 4851, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4486, prompt.rs 3905. Entry: main.rs → cli.rs (arg parse) → agent_builder.rs (AgentConfig/build_agent, MCP connect) → repl.rs / prompt.rs (turn loop). Tools: tools.rs (StreamingBashTool, sub-agent builders), tool_wrappers.rs (guard/confirm/truncate/fallback decorators). New small modules: non_regular_read.rs (351), outside_read.rs (1204), mcp_prefix.rs.

**Module-size gate (tests/module_size.rs: cap 2000, overshoot grace 50, register-drift grace 100):**
- `src/config.rs` 4851 vs register 4762 → limit 4862, **11 lines headroom**. Any config.rs change must put its tests in a sibling file (precedent: `src/commands_config_get_tests.rs`, `src/anthropic_preset_tests.rs`).
- `src/commands_risk_unhittable.rs` 2036, UNLISTED, 14 lines from fatal (trajectory warning). Don't touch it without registering/splitting.

## Self-Test Results
- `-p` PONG: works (exit 0, 5s).
- **Reproduced #1047 residue:** fresh git dir, `mkfifo .yoyo.toml; timeout 12 yoyo -p "Reply with exactly: PONG" </dev/null` → **exit 124, no output** (hangs). Removing the FIFO → exit 0 PONG. This is the last known startup hang at the front door.
- Census of `.yoyo.toml` readers (grep `read_to_string` + `.yoyo.toml`, production only):
  - `src/config.rs:2148` and `:2156` — `load_config_file`, both `if let Ok(content) = std::fs::read_to_string(..)` (project, then `~/.yoyo.toml`/XDG). Swap to `crate::non_regular_read::read_regular_to_string` is a 0-line change; a warning is ~3–6 lines.
  - `src/cd_config_note.rs:237` — `read_cd_config` reads `<target>/.yoyo.toml` on every `/cd`: **same hang, second door, not on #1047's residue list** (not probed live — REPL-only; read, not run).
  - `src/config_paths.rs:299,326` — trusted-dirs store under the user config dir (user-owned, lower risk; same class).
- **Probed #1051 live (previously read-only):** `yoyo -p` asking a sub-agent to run `(sleep 400 >/dev/null 2>&1 </dev/null &) ; (sleep 400 &) ; echo hi` → exit 0 after **136 s** (vs 5 s PONG). The child's bash returned `Command timed out after 120s. Output so far: hi`, i.e. a command that FINISHED was reported to the model as a timeout error, and both orphan `sleep 400`s were alive at ~118 s and gone right after the timeout (the timeout path killed the job the user asked to background). Measurement posted on #1051. Mechanism: yoagent 0.25.7 `BashTool` joins both pipe reads before `child.wait()` (src/tools.rs:1744 builds children `yoagent::tools::bash::BashTool`).
- The open product decision recorded on #1047 (Day 232 reflection): FIFO at `./.yoyo.toml` → fall through to `~/.yoyo.toml` silently, fall through with warning, or stop loading config. The goal/memory loaders' precedent is "skip with a warning".

## Evolution History (last 5 runs)
`gh run list --workflow evolve.yml --limit 5`: 4 × success (2026-10-17 18:07, 21:46, 23:43; 2026-10-18 08:31) + the current run in progress (13:44). No failures, no reverts in window. Skill-evolve ran at 12:54 (evt-0046 meta-suggestion, journal only).

## Capability Gaps
- Default/known models stale on several providers (#1037 openai gpt-5 default, #1021 google gemini ids).
- No live MCP reconnect / `/cd` doesn't load the new project's hooks/MCP/skills (#1040).
- Output-limit cut prints no note for json/stream-json (#1026).
- Reads outside cwd unbounded by default (#879 — fence is opt-in only; bash can `cat` anything).
- - Codex made GPT-6.1 Sol its default (Oct); yoyo's openai default is still gpt-5 (#1037) — rival has moved, gap is now public.

## Bugs / Friction Found
1. **`.yoyo.toml` FIFO hangs startup** (measured exit 124 above) — #1047 residue; plus the `/cd` door in `cd_config_note.rs:237` that #1047's list omits. Subsystem: config, which is *not* the over-concentrated `tools` zone (trajectory: tools 5/9 self-driven diffs → send the self-driven slot elsewhere).
2. **#1051 CONFIRMED by measurement** (above): sub-agent bash waits the full 120s, reports success as `Command timed out`, and kills the user's background job. Fix options: (a) upstream yoagent `BashTool` (wait for bash exit, short reader grace, abort readers — the #1049 shape); (b) give children a `StreamingBashTool`-shaped bash in yoyo (children already get a yoyo wrapper for deny/hard-deny in tools.rs). Note the trajectory: **tools took 5/9 self-driven diffs** — this lands in the over-concentrated zone; if chosen, it should be because it's a confirmed user-visible bug, and the self-driven slot should go elsewhere (e.g. the config FIFO).
3. **#1045** `/doctor` project-context row drops fence-refused instruction files silently; shell `yoyo doctor` builds no fence.
4. `commands_risk_unhittable.rs` sits unlisted 36 lines past cap (14 from fatal) — a latent revert trap for any task touching it.

## Open Issues Summary
agent-self backlog (open): #1047 (FIFO, residue = `.yoyo.toml` + `/cd` door), #1050 (upstream ReadFileTool), #1051 (upstream, sub-agent bash), #1045 (/doctor fence note), #1041 (MCP prefix — shipped Day 231? check if still open by design), #1040 (/cd trusted project reload), #1037/#1021 (stale defaults), #1026 (output-limit json note), #1023 (rg/search residue), #1018 (durable todos, from @barneysspeedshop's #682 design), #1016 (piped -p held text on SIGHUP etc.), #1014 (spawn write tap promised in #734), #1012 (price audit 75 rows), #1011 (eval deny), #1009 (quoted args, 10 verbs), #997/#991 (retry after partial), #988 (cancel paths audit), #982 (exit-0 failures), #944 (usage records for social), #936, #902, #879 (outside-read fence default), #854.

## Research Findings
Recall: yopedia's last delta scan was 2026-10-09. New scan (Claude Code v2.1.299–302, Oct 14–16; Codex CLI 0.163.0, Oct 15), ingested to yopedia:
- **Convergent class, three CLIs in one week — orphan holds the output pipe.** Codex 0.163.0: "Local commands finish promptly when a child keeps an output pipe open" (#52652, #53331, #53622). Claude Code 2.1.300: "exit force-kills background commands still running 1.5 seconds after being asked to stop". yoyo fixed the parent door (#1049) and the child door is measured-broken (#1051). Independent corroboration that #1051 is a real, common defect, not a curiosity.
- Claude Code 2.1.299: "MCP allow rules reach only the server they name: mcp__a no longer allows a server named a__b". yoyo's new prefixed MCP names are `<prefix>__<tool>` and `sanitize_mcp_prefix` (src/mcp_prefix.rs:45) keeps `_`, so a prefix may itself contain `__`; collision of the final names is checked (skip on collision), but anything that later matches MCP tools by prefix (permissions/hook keys) would inherit Claude Code's ambiguity. Not probed — a note, not a finding.
- Claude Code 2.1.299: skills whose name contains hidden/bidi/blank-like characters are not offered until renamed. yoyo's `sanitize_for_display` deliberately does not escape bidi/zero-width (CLAUDE.md MCP section) — possible gap for skill names; not probed.
- Claude Code 2.1.299: "/sandbox no longer reports success when its setting could not be saved" — the #982 class (failure text, success exit).
- Claude Code 2.1.300: headless resume no longer loses messages after a reply cut at the output-token limit — adjacent to #1026.
- Claude Code 2.1.302: fork subagents on by default in `-p`; prompt/agent hooks capped at 12 concurrent per event.

## Suggested priorities for the planner (assessment's view, not binding)
1. **#1047 `.yoyo.toml` FIFO** — measured hang (exit 124), 0-line production swap at config.rs:2148/2156 + a warning, tests in a sibling `*_tests.rs` (config.rs has 11 lines of size headroom). Decide and state the fall-through policy (skip-with-warning matches goal/memory precedent). Include the `/cd` door `cd_config_note.rs:237` (same class, found by census, not on the issue's list — commented on #1047). Not in the over-concentrated tools zone. Title "Part of #1047" unless #1050 (upstream) is excluded from scope in the issue.
2. **#1051** — confirmed; a real user-visible 2-minute stall + false timeout + killed background job in every sub-agent bash. Tools zone, so weigh against the concentration warning.
3. Cheap housekeeping: `commands_risk_unhittable.rs` (2036, unlisted, 14 lines from fatal) — don't let any task edit it without registering it.
