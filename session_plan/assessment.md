# Assessment — Day 230 (11:14)

## Build Status
Pass — verified by the harness at session start (HEAD c357f373). Not re-run here.
Binary probes from a clean `/tmp/probe` (no config): `-p "Reply with exactly PONG"` → `PONG`, exit 0;
`--version` → `yoyo v0.2.0 (c357f373 2026-10-16) linux-x86_64`; `yoyo grep zz9 /nonexistent` → error, exit 2;
`yoyo find main /nonexistent` → "find takes ONE pattern…", exit 2 (the #1009/#982 fixes hold).

## Recent Changes (last 3 sessions)
- **Day 230 08:44** (2/2): #1032 — `/cd` back into the launch dir no longer falsely warns its own `.yoyo.toml` is NOT applied (records the startup-loaded path, compares on return; unresolvable paths keep warning). #1034 — malformed `.yoyo.toml` (unterminated quote / unclosed `[`) now prints one `warning:` with file:line, wired into three readers (main loader, `config get`, …). Stray quote still sent, exit still 0 → residue filed as **#1036**.
- **Day 230 07:14** (1/2, one task reverted for touching protected `skills/evolve/SKILL.md`): yoagent 0.25.2 → 0.25.5, all 15 version-claim markers re-read. Yuanhao then landed the #1033 rule himself (8a122b32) and #1031 (upstream-label filing, c357f373), plus `evolve.sh: say who ran the suite`.
- **Day 230 00:04** (2/2): #1029 streamed markdown no longer leaks `##` / `**…**` across chunk splits; #869 slice — launch-project hooks pause after `/cd` elsewhere and resume on return.
- Day 229 (4 sessions, all 2/2): output-limit cut warning (#1026 residue), `/cd` allow-list suspension in the authority, rg-config fence hardening (#1023), `yoyo setup` refuses non-tty stdin, bad numeric config warns (#1028 → residue #1030), refusal exit codes (#982 slice).
- Deps bump 0379112b (tokio 1.53.3, regex 1.13.1, …).

## Source Architecture
125 files in `src/`, ~206.6K lines total (incl. tests). Largest: cli.rs 7686, commands_risk.rs 6602, tool_wrappers.rs 5674, tools.rs 5221, agent_builder.rs 4921, config.rs 4758, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4418, prompt.rs 3905, symbols.rs 3804. Entry: `main.rs` → `cli::parse_args` → `agent_builder` (build_agent, connect_external_servers) → `prompt.rs` run loops / `repl.rs`; shell verbs via `dispatch_sub.rs`; config via `config.rs` + `config_syntax.rs` (new); `/cd` policy split across `cd_config_note.rs`, `cd_allow.rs`, `cd_hooks.rs`, `cd_deny.rs`.

**Size-gate tripwire:** `src/commands_risk_unhittable.rs` = **2036 lines, unlisted, 36 over the 2000 cap; grace is 50** → 14 more lines makes `cargo test` fatal and reverts whatever task adds them. Any task touching that file must split it or register it first.

## Self-Test Results
- PONG / version / grep / find all correct (above).
- **Malformed config (#1036 reproduced):** `model = "claude-haiku-5-5` → warning names `.yoyo.toml:1: unterminated quote` and says "affected values are read as written, stray quotes included"; `config get model` exit 0. The warning is now honest about the residue, but the decision #1036 asks for is unmade.
- **Stale product default:** with no config, `-p` banner reports `model: claude-opus-4-6`. `default_model_for_provider` (`src/providers.rs:168-195`) falls back to `claude-opus-4-6`; docs (`docs/src/configuration/models.md:7,35`, `getting-started/installation.md:41`) say the same. The known-model list and presets already include claude-opus-5 / 5-5 / fable-5 / opus-4-8, and the loop itself runs opus-5-5. Not filed anywhere (searched "default model opus": only #1021, the google twin). Same class as #1021 (google default stale). Openrouter/bedrock defaults are `claude-sonnet-4-6`. Whether the newest is the *right* default (price) is a decision, not a fact — but nothing records it as a decision.

## Evolution History (last 5 runs)
11:12 in progress (this one); 08:42 success; 07:12 success (but 1 task reverted inside it — protected-file scope gate); 00:09 **cancelled** (overlapping start 7 min after the 00:02 run, which succeeded — concurrency cancel, not a failure); 00:02 success. Trajectory: 0 reverts in last ~10 sessions besides the 07:14 scope revert ("did not reach a verdict"); no CI failures in 14 days; no provider errors; usage records on 10/10.

## Capability Gaps
- Default model lags the newest Anthropic model (product first-run experience).
- `/cd` still doesn't reload the new folder's hooks, allow list, MCP servers, skills (#869, 4 items).
- Durable todos with due dates (#1018, designed with @barneysspeedshop/@danstis in #682) — unbuilt.
- No reconnection of surviving MCP servers after a failed connect (documented residue).
- Research section below.

## Bugs / Friction Found
1. **#1036** — malformed `.yoyo.toml`: warning ships, stray quote still sent, exit 0. Three consecutive journals ended on this question; the issue requires a decision (refuse / strip / keep-with-reason) pinned by test. Nobody reaches this branch on purpose (Day 229 rule).
2. **#1030** — `cost_warn_usd` typo silently turns the budget OFF on every door (flag, env, config); `max_auto_continues` same. Cheap, same shape as #1028.
3. **#1026** — json/stream-json print no output-limit note; exit-0 justified by the wrong population.
4. Stale anthropic default `claude-opus-4-6` (unfiled, see Self-Test).
5. commands_risk_unhittable.rs size tripwire (above).

## Open Issues Summary
agent-self backlog: #1030, #1026, #1023 (rg footer count / find warnings residue), #1021 (google stale models), #1018 (durable todos), #1016 (piped -p exit census), #1014 (spawn write tap, promised to @barneysspeedshop), #1012 (75 price-drift rows), #1011 (eval-assembled deny bypass), #1009 (quote_args residue), #997/#991 (retry-after-partial, retry_safe_events), #988 (cancel paths after 0.23), #982 (exit codes residue: bare run, undo --all, 1-vs-2 usage codes, goal verbs), #944, #936, #902, #879, #869. Unlabelled: **#1036** (filed by reflection 10:21Z today).

## Research Findings
(pending — see update)
