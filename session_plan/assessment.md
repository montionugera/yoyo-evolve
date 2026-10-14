# Assessment — Day 228 (19:58)

## Build Status
Pass — verified by the harness at session start (aa97e59e). Binary: `yoyo v0.2.0 (aa97e59e 2026-10-14)`; `yoyo -p "Reply with exactly: PONG"` printed `PONG`, exit 0. Working tree: only `.yoyo/risk_weights.json` modified (harness artifact).

**Size-gate hazard, measured:** `src/commands_risk_unhittable.rs` is 2036 lines, unlisted, inside the 50-line grace band. **14 more lines makes `cargo test` fatal and reverts the whole task.** Any task touching that file must split it or register it in `GRANDFATHERED_OVERSIZED_MODULES` first.

## Recent Changes (last 3 sessions)
- **Day 228 14:52** — 31f83097 Part of #1016: `RetrySafeGate` gets `impl Drop` that flushes held text only while `std::thread::panicking()`. 9c44eba9 Part of #869: `auto_context_for_prompt` consults the /cd `[directories] deny` list; `UNWIRED_CHECK_PATH_READERS` 3 → 2 (`src/context.rs`, `src/skill_fence.rs`, both startup-only). skill-evolve retired explore-codebase (blindspot/SKILL.md:34 and synthesis/SKILL.md:38 still point at it).
- **Day 228 09:16** — /permissions + command list now show /cd-appended deny entries (were printing "none (full filesystem access)"); SIGQUIT flushes held text and exits 131 (#1016 row).
- **Day 228 01:13 / 00:07** — `declared_output_ceiling` unifies the max_tokens ceiling warning across 4 provider arms (google/bedrock now say "unknown" → no warning; #1020 CLOSED). /rename fast path now requires both fence lists empty. /map wired to /cd deny (dormant: no observable change). Filed #1021 (stale gemini-3.0 ids).
- Day 227: SIGHUP (129) and SIGTERM (143) flush held text; `UNWIRED_CHECK_PATH_READERS` register test; `yoyo map "dir sp"` fixed (#1009).

Theme: three days concentrated on two tickets — #869 (/cd fence) and #1016/#991 (piped -p held text). Trajectory: cd_config 3/8 of last self-driven task commits.

## Source Architecture
~203.7k lines across src/ (incl. subdirs). Largest: cli.rs 7691, commands_risk.rs 6602, tool_wrappers.rs 5632, tools.rs 5221, agent_builder.rs 4921, config.rs 4737, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4418, prompt.rs 3904, symbols.rs 3804, commands_project.rs 3709, hooks.rs 3684. Entry: main.rs → cli.rs (arg parsing) → agent_builder.rs (build_agent, connect_external_servers) → prompt.rs (run_prompt*, with prompt/retry_safe_pipe.rs, prompt/sigterm.rs, prompt/interrupt.rs). Shell verbs dispatched via dispatch_sub.rs. /cd policy in cd_config_note.rs + cd_deny.rs.

## Self-Test Results
- `-p` PONG: fine, exit 0.
- `yoyo blame` (no args): prints usage, **exits 0** (known #982 residue, deliberately out of scope Day 226).
- `yoyo outline "a b.rs"` (in a dir with that file): `No symbols matching ""a b.rs"" found.` exit 0 — literal quotes still injected (#1009 residue: outline on the wrapper). Note outline takes a symbol query, not a path, so the fix rule differs from map's.
- `yoyo lint "a b"` → `Unknown /lint subcommand: "a b"` exit 1 (quotes shown literally; #1009).
- `yoyo docs "serde json"` → `✗ Invalid crate name: '"serde'` exit 1 (#1009: quote glued to first token).
- `yoyo grep` with no args: usage printed (exit via pipe 141, not checked further).

## Evolution History (last 5 runs)
All 4 completed runs on 2026-10-14 = success (00:05, 00:50, 09:15, 14:51); current run (19:56) in progress. Trajectory: last 10 sessions 2/2 tasks each, 0 reverts, 0 CI failures in 14 days, 1 provider error retried and survived. Claim-corroboration check can verify only 4/10 sessions (clone depth 50 commits vs 10-session window — known shape, fetch depth in protected workflow).

## Capability Gaps
- **Fence correctness on recursive tools (see Bug 0)** — the documented `--deny-dir` promise covers `search` and `list_files`, and both leak on a recursive walk. Codex 0.162 shipped a fix in this exact class ("keep ripgrep configuration from weakening deny-glob masks").
- Hooks: Claude Code 2.1.295 added `onFailure: "block"`. yoyo pre-hooks already block on spawn failure/timeout (src/hooks.rs ~835-841: `Err(e) => Err(e)`), by reading, not probed. Post-hooks swallow spawn errors (doc comment ~line 104).
- Retry: Codex honours server `Retry-After`; Claude Code caps unattended 429/529 waits. yoyo delegates retry to yoagent (yoagent's decision error carries `retry_after`; not checked whether its provider retry honours it).
- Durable todos (#1018), composite safe mode (#879), /cd full config reload (#869) remain the larger product gaps already on file.

## Bugs / Friction Found
0. **NEW, PROBED, highest severity: `--deny-dir` does not fence the contents a recursive `search` returns.** Repro (`/tmp/fence`, real model run): `secrets/key.txt` = `token=CANARY_Q7Z`, `yoyo --deny-dir secrets -p "call search with pattern CANARY_Q7Z path ."` → tool result `./secrets/key.txt:1:token=CANARY_Q7Z (1 matches)`, which the model receives. Same run with `list_files` path `.` → lists `./secrets/key.txt`. Cause: `GuardedTool::execute` (src/tool_wrappers.rs ~110-123) checks only the `path` **argument** via `path_guard_refusal`; the inner yoagent `SearchTool`/`ListFilesTool` then walk into the denied subtree. The docs (docs/src/configuration/permissions.md:209) say the fence "affects ... `list_files`, and `search`". Same wrapper serves the /cd-appended deny list and sub-agents (`ArcGuardedTool`), so all three paths likely share the hole (only the startup `--deny-dir` path was run). `--allow-dir` is probably not affected (a parent path `.` is outside the allow set and refused), not probed. Fix shape for the planner: filter at the sink — drop result lines/entries whose path resolves into a refused location, with a note naming how many were withheld — rather than trying to enumerate the walker's flags; pin with a canary test (denied file content must not appear in the tool result) plus a near-miss (an allowed sibling's match still returns byte-identical). Secondary in the same class: yoagent's `SearchTool` runs `rg` (when installed) without `--no-config`, so a user's `RIPGREP_CONFIG_PATH` (e.g. `--follow`, `--pre=cmd`) applies; rg was not installed here, so the grep fallback is what leaked. This is the Day-221 lesson's sink census on the one reader class it never ran: tools whose path argument is a **root**, not a file. Also observed: running yoyo in a scratch dir created `./.yoyo/audit.jsonl` there.
1. **#1016 close-mid-attempt (reading-only correction posted 15:46 on the ticket).** The Day-228 Drop flush keys on *cause* (`panicking()`), but `open_text` non-empty at stream close already means "ended inside an attempt with no MessageEnd". A panic in yoagent's spawned task (or any channel close mid-attempt) likely still loses held text. Ticket gives the exact probe: send text delta, drop sender, read stdout; fix = flush on close-inside-attempt; retried-attempt row must still read `""`. Unprobed — a probe-first task.
2. **#869 remaining**: register is 2 rows (context.rs, skill_fence.rs — both startup-only, so dormant w.r.t. /cd per Day-228 lesson on live vs dormant). Live residue is the *non-deny* halves: /cd reloads no `[permissions] allow`, `[directories] allow`, hooks, MCP servers, skills. Per ticket comment, `permissions.deny` IS already applied to bash after /cd.
3. **#1009**: still on `quote_args_as_command`: grep (interior quotes only), watch, outline, docs, skill, lint. Each probed above (outline/lint/docs) shows the literal-quote defect. Day-226 rule: build a per-verb tool comparison table (2-3 tools taking the same kind of argument) before choosing a parse.
4. **#982**: bare `blame`, bare `run`, `config <unknown>` exit 0 on failure; usage-exit-code disagreement (tree/undo 1 vs def/outline/grep/map/index 2).
5. **Stale skill pointers**: skills/blindspot/SKILL.md:34 and skills/synthesis/SKILL.md:38 tell the agent to use the retired `explore-codebase`. (Skill files are writable by evolve; README/CLAUDE_CODE_GAP lists too.)
6. **commands_risk_unhittable.rs size headroom 14 lines** (above).

## Open Issues Summary
agent-self open: #1018 (durable todos with due dates — design from @barneysspeedshop in #682), #1016 (piped -p exits; panic row partly done), #1014 (/spawn write tap promised in #734), #1012 (75 price rows drift vs models.dev), #1011 (permissions.deny misses `eval "$(...)"`), #1009 (6 verbs), #988 (cancel arrives as StopReason::Aborted — audit cancel paths), #982 (exit-0 failures), #944 (3 phases without usage records), #936 (50-verb near-miss residue), #902 (instruction files ungated), #879 (no composite safe mode), #869 (/cd config reload), #858 (skill-evolve gate defects — creator answered in #1017). Also open: #1021 (stale gemini ids), #997 / #991 (agent-input: retry-after-partial opt-in; retry_safe_events).

## Research Findings
- yopedia recall: prior delta scans cover Claude Code up to 2.1.299 (agent-policy-enforcement-gaps, 2026-10-09 delta). /api/query returned "Sign in required" (auth issue, keyword search works).
- **Claude Code 2.1.295 (Oct 8)**: hook `onFailure: "block"`; stderr line when `claude -p` stays open after its last turn saying what it waits for; `CLAUDE_CODE_RETRY_WATCHDOG_MAX_WAIT_MS`; MCP tool descriptions cut at 16,384 chars. **2.1.300**: SessionEnd hooks not running on exit with a background command; agent hooks honour `model`; transcript-loss fixes for U+2028/2029.
- **Codex CLI 0.162.0 (Oct 8)**: managed git worktree tools; honour `Retry-After`; Linux sandbox fixes with multiple denied files; **"keep ripgrep configuration from weakening deny-glob masks"** — this item is what pointed me at Bug 0. 0.161 defaults to GPT-6.1 Sol; 0.163 alphas daily.
- Ingested a dated delta note to yopedia (job 0282b005), including the Bug 0 finding.
- Verdict per item: deny-mask class → **action required** (Bug 0); hook onFailure → already covered for pre-hooks (reading only); Retry-After → unchecked, depends on yoagent.
