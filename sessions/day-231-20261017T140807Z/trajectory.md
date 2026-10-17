# YOUR TRAJECTORY

Last computed: 2026-10-17T13:04Z. Day 231. Window: last 10 sessions / 14 days.

## Recent session outcomes (last 10)
day-231 (2026-10-17 11:23:28): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 09:09:29): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 01:44:32): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-17 00:09:54): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-16 22:36:19): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-16 20:59:02): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-16 15:29:26): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-16 12:43:13): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-16 10:22:47): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-16 08:30:23): tasks 1/2 ⚠️ — 1 task(s) did not reach a verdict (tree green, no revert recorded)
Claim corroboration: 0 of 5 checkable claiming sessions claimed success with no task commits.
5 further claiming session(s) could NOT be checked — NOT counted above: 4 window opens before this shallow clone's oldest commit; 1 oldest session read, no predecessor to open its window.

## Per-task activity (last 14 days)
"Part of #1030: a typo'd cost_warn_usd / --cost-warn / YOYO_C…": 2 attempt(s), last day-230
"Fixes #1042: /spawn's instruction-file loader honours the /c…": 1 attempt(s), last day-231
"Part of #1041: connect an MCP stdio server that collides on …": 1 attempt(s), last day-231
"Fixes #869: measure whether the launch project's MCP servers…": 1 attempt(s), last day-231
"Part of #1023: search's "(N matches)" footer counts only wha…": 1 attempt(s), last day-231
"Part of #869: /cd into a trusted dir no longer claims its MC…": 1 attempt(s), last day-231

## Reverts in window
0 task reverts in last ~10 sessions. 1 task(s) did not reach a verdict (tree green, no revert recorded) across 1 of the last ~10 sessions.
0 whole-session revert commit(s) in last 14 days.

## Subsystem concentration (last 10 self-driven task commits)
cd_config: 4/10
cli: 4/10
fence_result: 2/10
prompt: 2/10
agent: 1/10
(+12 other subsystem(s) with fewer)

## CI: no failed runs in last 14 days (5 older failure(s) outside the window, not shown)

## Provider/API health
10 sessions, no provider-error lines in audit.jsonl, transcripts/*.log.

## Usage records
10 of 10 sessions carry >=1 usage record (#848 channel is live).

## Module sizes (the size gate warns but only fails on the exit code)
src/commands_risk_unhittable.rs is 2036 lines, 36 past the 2000-line cap and UNLISTED — 14 more line(s) makes `cargo test` FATAL, which reverts the whole task. Fix: split it, or add ("src/commands_risk_unhittable.rs", 2036) to GRANDFATHERED_OVERSIZED_MODULES.
src/config.rs is 4851 lines vs its recorded 4762 (+89 drift) — 11 more line(s) makes it FATAL. Fix: paste ("src/config.rs", 4851) over its entry.

## Doc freshness (the header, not the body)
... (truncated to fit token budget)
