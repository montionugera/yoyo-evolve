# YOUR TRAJECTORY

Last computed: 2026-10-18T18:34Z. Day 232. Window: last 10 sessions / 14 days.

## Recent session outcomes (last 10)
day-232 (2026-10-18 15:01:28): tasks 2/2 ✅ — build OK, tests OK
day-232 (2026-10-18 09:44:42): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-18 00:45:46): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 23:05:08): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 19:11:39): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 14:08:07): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 11:23:28): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 09:09:29): tasks 2/2 ✅ — build OK, tests OK
day-231 (2026-10-17 01:44:32): tasks 2/2 ✅ — build OK, tests OK
day-230 (2026-10-17 00:09:54): tasks 2/2 ✅ — build OK, tests OK
Claim corroboration: 0 of 4 checkable claiming sessions claimed success with no task commits.
6 further claiming session(s) could NOT be checked — NOT counted above: 5 window opens before this shallow clone's oldest commit; 1 oldest session read, no predecessor to open its window.
Shape, not a per-run accident: 5 of 10 claiming sessions open before the clone. Window = last 10 sessions; clone = 50 commits, oldest 2026-10-17 17:20 UTC; 4 of the 10 sessions read open their window after it. The window is this script's; the fetch depth is set in .github/workflows/evolve.yml (protected).

## Per-task activity (last 14 days)
"Part of #1047: a FIFO at .yoyo.toml no longer hangs startup …": 2 attempt(s), last day-232
"Fixes #1049: yoyo -p exits promptly when a backgrounded gran…": 2 attempt(s), last day-232
"Part of #879: the outside-read fence anchors on the launch d…": 2 attempt(s), last day-231
"Part of #1047: read_file refuses a FIFO, device or other non…": 2 attempt(s), last day-231
"Part of #982: unknown verbs for skill, goal and todo exit 2 …": 1 attempt(s), last day-232
"Part of #1047: a FIFO at .yoyo/goal.md or .yoyo/memory.json …": 1 attempt(s), last day-232

## Reverts in window
0 task reverts in last ~10 sessions, 0 whole-session revert commits in 14 days.

## Subsystem concentration (last 9 self-driven task commits)
tools: 4/9
non_regular: 3/9
outside: 3/9
goal: 2/9
main: 2/9
(+7 other subsystem(s) with fewer)

## CI: no failed runs in last 14 days (5 older failure(s) outside the window, not shown)

## Provider/API health
10 sessions, no provider-error lines in audit.jsonl, transcripts/*.log.

## Usage records
10 of 10 sessions carry >=1 usage record (#848 channel is live).

## Module sizes (the size gate warns but only fails on the exit code)
src/commands_risk_unhittable.rs is 2036 lines, 36 past the 2000-line cap and UNLISTED — 14 more line(s) makes `cargo test` FATAL, which reverts the whole task. Fix: split it, or add ("src/commands_risk_unhittable.rs", 2036) to GRANDFATHERED_OVERSIZED_MODULES.
src/config.rs is 4851 lines vs its recorded 4762 (+89 drift) — 11 more line(s) makes it FATAL. Fix: paste ("src/config.rs", 4851) over its entry.

... (truncated to fit token budget)
