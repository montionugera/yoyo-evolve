# YOUR TRAJECTORY

Last computed: 2026-10-16T08:53Z. Day 230. Window: last 10 sessions / 14 days.

## Recent session outcomes (last 10)
day-230 (2026-10-16 08:30:23): tasks 1/2 ⚠️ — 1 task(s) did not reach a verdict (tree green, no revert recorded)
day-230 (2026-10-16 01:29:44): tasks 2/2 ✅ — build OK, tests OK
day-229 (2026-10-15 20:58:40): tasks 2/2 ✅ — build OK, tests OK
day-229 (2026-10-15 16:02:06): tasks 2/2 ✅ — build OK, tests OK
day-229 (2026-10-15 10:15:04): tasks 2/2 ✅ — build OK, tests OK
day-229 (2026-10-15 01:04:21): tasks 2/2 ✅ — build OK, tests OK
day-228 (2026-10-14 21:23:23): tasks 2/2 ✅ — build OK, tests OK
day-228 (2026-10-14 15:50:39): tasks 2/2 ✅ — build OK, tests OK
day-228 (2026-10-14 10:16:46): tasks 2/2 ✅ — build OK, tests OK
day-228 (2026-10-14 02:11:59): tasks 2/2 ✅ — build OK, tests OK
Claim corroboration: 0 of 4 checkable claiming sessions claimed success with no task commits.
6 further claiming session(s) could NOT be checked — NOT counted above: 5 window opens before this shallow clone's oldest commit; 1 oldest session read, no predecessor to open its window.
Shape, not a per-run accident: 5 of 10 claiming sessions open before the clone. Window = last 10 sessions; clone = 51 commits, oldest 2026-10-15 10:14 UTC; 4 of the 10 sessions read open their window after it. The window is this script's; the fetch depth is set in .github/workflows/evolve.yml (protected).

## Per-task activity (last 14 days)
"Fixes #1029: streamed markdown no longer leaks `##` heading …": 2 attempt(s), last day-230
"Fixes #1028: a bad numeric value in .yoyo.toml (max_tokens, …": 2 attempt(s), last day-229
"Part of #869: after /cd, the launch project's [permissions] …": 2 attempt(s), last day-229
"A reply cut off at the output-token limit says so on stderr …": 2 attempt(s), last day-229
"Upgrade yoagent 0.25.2 → 0.25.5: re-read the 15 version-clai…": 1 attempt(s), last day-230
"Part of #869: after /cd into another project, the launch pro…": 1 attempt(s), last day-230

## Reverts in window
0 task reverts in last ~10 sessions. 1 task(s) did not reach a verdict (tree green, no revert recorded) across 1 of the last ~10 sessions.
0 whole-session revert commit(s) in last 14 days.

## Subsystem concentration (last 7 self-driven task commits)
cli: 3/7
cd: 2/7
config: 2/7
dispatch: 2/7
format: 2/7
(+9 other subsystem(s) with fewer)

## CI: no failed runs in last 14 days (5 older failure(s) outside the window, not shown)

## Provider/API health
10 sessions, no provider-error lines in audit.jsonl, transcripts/*.log.

## Usage records
10 of 10 sessions carry >=1 usage record (#848 channel is live).

## Module sizes (the size gate warns but only fails on the exit code)
src/commands_risk_unhittable.rs is 2036 lines, 36 past the 2000-line cap and UNLISTED — 14 more line(s) makes `cargo test` FATAL, which reverts the whole task. Fix: split it, or add ("src/commands_risk_unhittable.rs", 2036) to GRANDFATHERED_OVERSIZED_MODULES.
... (truncated to fit token budget)
