# YOUR TRAJECTORY

Last computed: 2026-10-14T09:19Z. Day 228. Window: last 10 sessions / 14 days.

## Recent session outcomes (last 10)
day-228 (2026-10-14 02:11:59): tasks 2/2 ✅ — build OK, tests OK
day-228 (2026-10-14 01:05:39): tasks 2/2 ✅ — build OK, tests OK
day-227 (2026-10-13 22:28:20): tasks 2/2 ✅ — build OK, tests OK
day-227 (2026-10-13 12:51:40): tasks 2/2 ✅ — build OK, tests OK
day-226 (2026-10-12 23:46:17): tasks 2/2 ✅ — build OK, tests OK
day-226 (2026-10-12 12:44:51): tasks 2/2 ✅ — build OK, tests OK
day-226 (2026-10-12 07:23:10): tasks 2/2 ✅ — build OK, tests OK
day-226 (2026-10-12 02:39:35): tasks 2/2 ✅ — build OK, tests OK
day-226 (2026-10-12 01:13:32): tasks 2/2 ✅ — build OK, tests OK
day-225 (2026-10-11 21:13:05): tasks 2/2 ✅ — build OK, tests OK
Claim corroboration: 0 of 4 checkable claiming sessions claimed success with no task commits.
6 further claiming session(s) could NOT be checked — NOT counted above: 5 window opens before this shallow clone's oldest commit; 1 oldest session read, no predecessor to open its window.
Shape, not a per-run accident: 5 of 10 claiming sessions open before the clone. Window = last 10 sessions; clone = 50 commits, oldest 2026-10-12 23:46 UTC; 4 of the 10 sessions read open their window after it. The window is this script's; the fetch depth is set in .github/workflows/evolve.yml (protected).

## Per-task activity (last 14 days)
"Fixes #1020: no "declared output ceiling" warning on the bed…": 2 attempt(s), last day-228
"Part of #869: /rename's file filter consults the /cd deny li…": 1 attempt(s), last day-228
"Part of #869: the repo map's fence check consults the /cd [d…": 1 attempt(s), last day-228
"Fixes #1019: no "declared output ceiling" warning when no pr…": 1 attempt(s), last day-228
"Part of #869: pin "no check_path reader skips the /cd deny l…": 1 attempt(s), last day-227
"Part of #1016: a SIGHUP'd piped `-p` run flushes the held te…": 1 attempt(s), last day-227

## Reverts in window
0 task reverts in last ~10 sessions, 0 whole-session revert commits in 14 days.

## Subsystem concentration (last 8 self-driven task commits)
cd_config: 3/8
agent: 2/8
main: 2/8
map: 2/8
prompt: 2/8
(+6 other subsystem(s) with fewer)

## CI: no failed runs in last 14 days (5 older failure(s) outside the window, not shown)

## Provider/API health
10 sessions, 1 provider error hit(s) in audit.jsonl, transcripts/*.log. 0 session(s) ended on a terminal give-up — every hit above was retried (survived, not died). (1 prose-shaped line(s) rejected)

## Usage records
10 of 10 sessions carry >=1 usage record (#848 channel is live).

## Module sizes (the size gate warns but only fails on the exit code)
src/commands_risk_unhittable.rs is 2036 lines, 36 past the 2000-line cap and UNLISTED — 14 more line(s) makes `cargo test` FATAL, which reverts the whole task. Fix: split it, or add ("src/commands_risk_unhittable.rs", 2036) to GRANDFATHERED_OVERSIZED_MODULES.
... (truncated to fit token budget)
