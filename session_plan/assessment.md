# Assessment — Day 230 (08:44)

## Build Status
Pass — verified by the harness at session start (HEAD 0379112b, a semver-compatible deps bump on top of the 07:14 session). Binary: `yoyo v0.2.0 (0379112b 2026-10-16)`; `yoyo -p "Reply with exactly: PONG"` in an empty /tmp dir → `PONG`, exit 0.

## Recent Changes (last 3 sessions)
- **07:14 (Day 230):** Task 1 landed — yoagent 0.25.2 → 0.25.5, all 15 version-claim paragraphs re-read (b58e4329). Upstream 0.25.5 ships rg `--no-config` (our yoagent#315) and a search that ends badly now says so. Task 2 was **reverted by the harness**: it bundled the #1033 rule (an edit to the PROTECTED `skills/evolve/SKILL.md`) with the #1032 fix, so the unprotected #1032 fix died with it. The creator has since applied the #1033 rule himself (8a122b32, "Closes #1033") and **asked on #1032 to retry its fix as its own task**.
- **00:04 (Day 230):** #1029 streamed markdown `##`/`**` leak fixed; #869 slice: launch-project hooks pause after `/cd` away and resume on return.
- **19:41 (Day 229):** #1028 bad numeric `.yoyo.toml` values now warn (residue #1030); refusals needing a tty now exit 2 (#982 slice).
- 0379112b (harness): async-trait, regex, tokio 1.53.3, serde_json, etc. semver bumps.

## Source Architecture
~208k lines under src/ (incl. *_tests.rs). Largest: cli.rs 7686, commands_risk.rs 6602, tool_wrappers.rs 5674, tools.rs 5221, agent_builder.rs 4921, config.rs 4742, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4418, prompt.rs 3905. Entry: main.rs → cli::parse_args → agent_builder → prompt::run_prompt*; REPL dispatch in dispatch.rs / dispatch_sub.rs (shell subcommands). Size gate: `src/commands_risk_unhittable.rs` 2036 lines, UNLISTED, inside the 50-line grace band (fatal at 2051) — any task touching it must add the register entry or split.

## Self-Test Results
- `-p` PONG: works (model printed: `claude-opus-4-6`, the anthropic product default — see Bugs).
- **#1034 still reproduces at HEAD:** `.yoyo.toml` containing `model = "claude-haiku-5-5` (unterminated) → `yoyo config get model` prints `model = "claude-haiku-5-5  (.yoyo.toml)` and exits 0; no warning anywhere.
- **#1032 retry is ready:** the reverted diff `sessions/day-230-20261016T083023Z/task2_reverted.patch` on the `audit-log` branch (233 lines; ARCHITECTURE.md, docs/.../permissions.md, src/cd_config_note.rs, src/config.rs, plus skills/evolve/SKILL.md which must be EXCLUDED) applies cleanly to HEAD: `git apply --check --exclude='skills/*' /tmp/t2.patch` → clean. Fetch with `git fetch origin audit-log --depth=1 && git show FETCH_HEAD:sessions/day-230-20261016T083023Z/task2_reverted.patch`. Shape: `config::LOADED_CONFIG_RESOLVED` (resolved at load time while cwd is still the launch dir) + `loaded_config_resolved_path()`, and in cd_config_note.rs `is_loaded_config()` + `reading_for_note()` consulted by `print_cd_config_note`; ~90 lines of tests. The prior attempt's watch (clippy -D warnings + cargo test) passed with it. Its own transcript names residue: on return home, a launch file with deny entries still prints the "deny applied … would still not be reloaded on /cd" note (true but beside the point) — that stays on #869.

## Evolution History (last 5 runs)
evolve.yml: 08:42 (this run, in progress), 07:12 success (1/2 tasks; task 2 reverted by protected-file gate, not by tests), 00:09 cancelled (concurrency, superseded), 00:02 success, 19:40 success, 14:38 success. CI: no failures in 14 days. No provider errors in 10 sessions. Only revert in window = the protected-file bundle.
Pattern worth naming: the revert cost a correct, unprotected fix because two tasks shared one box and one of them targeted a protected file. Planner check: before titling a task, grep evolve.sh's protected list for every file the task's done-when names.

## Capability Gaps
- Config robustness: Claude Code (v2.1.299) now lists the file and invalid entry on stderr when a settings file fails validation under `-p`; yoyo still loads a malformed `.yoyo.toml` silently (#1034) and silently drops a bad `cost_warn_usd` (#1030). This is the most current, concrete, user-facing gap with an open ticket on each half.
- `/cd` reload (#869): CC fixed headless sessions starting a folder's `.mcp.json` after a directory change; yoyo's remaining #869 slices are the new folder's own hooks, allow list, MCP servers and skills.
- Hook failure semantics: CC added `onFailure: "block"` (2.1.295). Checked: yoyo's `ShellHook::pre_execute` already returns `Err` (blocks) on spawn failure/timeout (src/hooks.rs ~841-848) — no gap, nothing to do.
- Default model staleness: CC made Haiku 5.5 its default Haiku model (2.1.293); yoyo's anthropic product default is still `claude-opus-4-6`.

## Bugs / Friction Found
1. **#1032** (reopened by creator; fix exists, reverted only by bundling) — highest-value, lowest-risk: re-apply the saved patch minus skills/, run tests, commit `Fixes #1032` as its OWN task.
2. **#1034** malformed `.yoyo.toml` (unterminated quote / unclosed `[` header) loads silently; a model id with a stray `"` 404s. Re-confirmed above. Suggested shape (from the issue): keep lenient value, add one stderr warning naming file:line, sanitized via `cli::sanitize_for_display`; valid file → stderr byte-identical. Note `parse_config_file` is in config.rs (4742 lines; check its GRANDFATHERED entry before adding).
3. **#1030** typo'd `cost_warn_usd` silently turns the budget OFF; `max_auto_continues` same class. Small, sibling of the shipped #1028 helper (`cli_config::resolve_numeric_setting`).
4. Anthropic product default model is `claude-opus-4-6` (providers.rs:191 `_ =>` arm; openrouter/bedrock defaults also 4-6). It still works (PONG), so this is staleness, not breakage; it is a product-visible default and changing it is a priced decision (cf. #1021 for google). Not filed; low priority.

## Open Issues Summary
agent-self open: #1034, #1032, #1030, #1026 (json/stream-json no output-limit note), #1023 (rg config item now upstream in 0.25.5; stale "(N matches)" footer, find warnings naming denied path remain), #1021 (stale gemini defaults), #1018 (durable todos, design from #682), #1016 (piped -p held text on other exits), #1014 (spawn write tap, promised in #734), #1012 (price drift vs models.dev), #1011, #1009 (quote_args_as_command residue, 10 verbs), #997/#991 (agent-input: retry-after-partial / retry_safe_events), #988, #982 (exit-0 failures residue), #944, #936, #902, #879, #869 (/cd reload: new folder's hooks, allow list, MCP, skills).

## Research Findings
- Scanned Claude Code v2.1.296–v2.1.301 (Oct 9–15) via the raw `feed.xml`. Mapped items: (1) "-p saying nothing when a settings file fails validation" → #1034/#1030; (2) headless `.mcp.json` started after cd → #869; (3) `onFailure: block` hooks → already fail-closed in yoyo; (4) Bash permission auto-approving `BASH_ARGV0` assignment-then-use → same class as #1011 (deny can't see runtime-assembled commands); (5) Haiku 5.5 as default Haiku; (6) retry-watchdog max wait for usage limits ≈ yoyo `--wait-for-reset`.
- yopedia recall: prior "Agent Changelog Delta Analysis" notes hold the framing that a competitor changelog is a pre-graded ledger of real defect classes; today's delta saved as "AI Coding Agent Changelog Delta — 2026-10-16".

## Suggested priorities for the planner (assessment's view, not a plan)
1. **#1032 retry, alone in its box** — apply the saved patch with `--exclude='skills/*'`, run fmt/clippy/test, title `Fixes #1032` (the creator explicitly asked for this as its own task). config.rs is registered at 4742 and the patch adds ~20 lines there (inside the 100-line register-drift band; last attempt's full test run passed). Keep the "deny applied" note-on-return residue on #869 (say `Part of`/name it, don't close #869).
2. **#1034** malformed-syntax warning (file:line, sanitized, valid file → stderr byte-identical). Check before titling: no protected file in its done-when.
3. Alternatively **#1030** (sibling of the shipped #1028 helper) if #1034's parser change looks too wide.
Guardrail from this morning's revert: before titling any task, grep evolve.sh's protected list for every path the task's done-when names, and never put two issues in one task.
