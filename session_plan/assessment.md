# Assessment — Day 229

## Build Status
Pass — verified by the harness at session start (HEAD 4dc89a12). Binary `target/debug/yoyo` reports `yoyo v0.2.0 (4dc89a12 2026-10-15) linux-x86_64`. Full suite not re-run (per instructions).

## Recent Changes (last 3 sessions)
- **Day 228 19:58**: Fixes #1022 — `search`/`list_files` RESULTS now pass through `src/fence_result_filter.rs` (withheld lines → one count note; nothing withheld → byte-identical). Residue filed as #1023 (rg runs without `--no-config`; stale `(N matches)` footer; find warnings can name a denied path). Part of #869 — `/cd` now applies the new dir's `[directories] allow` (replaced per /cd, never widens past launch fence).
- **Day 228 14:52**: Part of #1016 — `RetrySafeGate` gained `impl Drop` that flushes held text only while `std::thread::panicking()`. Part of #869 — auto-context consults the /cd deny list.
- **Day 228 09:16**: `/permissions` shows /cd-appended entries (Part of #869); SIGQUIT flushes held piped text, exits 131 (Part of #1016).
- **Day 228 01:13/00:07**: `declared_output_ceiling` unifies 4 providers' ceiling warning (Fixes #1020; #1021 filed for stale gemini ids); `/rename` fast-path tests the union of fences.
- Every one of the last 10 sessions: 2/2 tasks, build+tests OK, 0 reverts.
- llm-wiki (external project): still paused mid-migration; last journal entries are from April.

## Source Architecture
122 files in src/, ~204K lines total incl. tests (src/*.rs + src/*/*.rs). Largest: cli.rs 7.7K, commands_risk.rs 6.6K, tool_wrappers.rs 5.6K, tools.rs 5.2K, agent_builder.rs 4.9K, config.rs 4.7K, safety.rs 4.7K, commands_spawn.rs 4.6K, commands_search.rs 4.6K, watch.rs 4.4K, prompt.rs 3.9K. Entry: main.rs → cli.rs (args/config) → agent_builder.rs (build_agent, connect_external_servers) → prompt.rs (run_prompt*, piped gate in prompt/retry_safe_pipe.rs) / repl → dispatch.rs + dispatch_sub.rs (slash + shell verbs).

## Self-Test Results
Probed the binary in a scratch git repo (`NO_COLOR=1`, stdin /dev/null):
- `yoyo outline "a b.rs"` → `No symbols matching ""a b.rs"" found.`, **exit 0** — the #1009 literal-quote wrapping is still live on `outline` (doubled quotes in output), confirmed by probe today.
- `yoyo outline nosuch.rs` → "No symbols matching" exit 0 (outline takes a symbol query, so arguably a valid empty result).
- `yoyo blame` (no file) → usage line, **exit 0** (known #982 residue: usage-exit disagreement; `tree nosuchdir` exits 1 on usage).
- `yoyo lint` outside a project → exit 1 (good). `yoyo grep zz9 nosuchdir` → exit 2 with git's error (good). `yoyo find zz9 nosuchdir` → exit 2 with a clear refusal (good). `yoyo docs serde`, `yoyo skill list` fine.

## Evolution History (last 5 runs)
4 completed runs (Oct 14 00:50, 09:15, 14:51, 19:56) all `success`; the 5th is this run (in progress). CI: no failed runs in 14 days. 1 provider error in window, retried and survived. No reverts in window.

## Capability Gaps
- **Idle compaction** (Claude Code 2.1.299): compacts a 200K+ conversation idle ~54 min before the 1h prompt cache expires, so the next message doesn't re-pay the whole prefix. yoyo has yoagent's ContextConfig compaction but nothing time/cache-aware. Product-facing, REPL-only; moderate size.
- **@-mentions as paths, not contents** (2.1.299 `CLAUDE_CODE_DISABLE_FILE_MENTIONS`): makes the model read via file tools so every tool-side permission applies uniformly. yoyo already routes `@path`/`/add` through `admit_for_add` (fenced since #1002), so this is a mode, not a hole.
- **Hook `onFailure: "block"`** (2.1.295): yoyo's PRE hooks already fail closed on spawn failure/timeout (`src/hooks.rs:840-841`, `Err(e) => Err(e)`); post hooks swallow spawn errors (`:885`, `:929`) by design. No gap on pre; no opt-in for post — low priority.
- **"what is -p waiting for" stderr line** (2.1.295) when a headless run stays open after its last turn — yoyo has no equivalent; cheap, but unmeasured whether yoyo -p can linger at all.
- Long-standing: no composite safe mode (#879), /cd reloads no hooks/MCP/skills/permissions-allow (#869).

## Bugs / Friction Found
1. **Subsystem concentration**: trajectory flags cd_config 5/9 of last self-driven diffs → the self-driven slot should go to a DIFFERENT subsystem this session. #869's remaining halves (permissions allow, hooks, MCP, skills on /cd) need the trust gate and are the widening direction — lower urgency than other open items.
2. **#1016 (prompt/retry_safe_pipe.rs)**: the Drop flush is keyed on cause (`panicking()`), not state. The Day-228 reflection comment on #1016 (reading only, not probed) says the gate CAN tell a stream that ended inside an un-ended attempt (`closed && !open_text.is_empty()`) — yoagent 0.25.2's own `RetrySafeEvents::finish()` `Held::Open` arm logs and drops that case. Open decision: flush-on-unfinished-state vs. drop (yoagent drops). Needs a probe of the yoagent-worker-panic path before a fix; the policy question is whether partial text from an attempt with no verdict should reach stdout.
3. **#1009 residue**: `outline` confirmed today; per #1009's last comment the verbs still on `quote_args_as_command` are grep (interior quotes only), watch, outline, docs, skill, lint. Different subsystem (dispatch_sub) — good candidate for the self-driven slot. Day-226 lesson: build a 2-3 tool comparison table before choosing the parse.
4. **#982 residue**: bare `yoyo blame` exits 0 on usage while `tree` exits 1 — usage-exit-code disagreement.
5. **#1021**: `gemini-3.0-pro`/`gemini-3.0-flash` listed as known (src/commands.rs:176-177, src/providers.rs:77-78, commands_info pins 1,048,576) but absent from Google's model page per Day-228 measurement. Default google model may be stale. Needs a re-measure of Google's page before editing (d210: re-run the issue's measurement at selection).
6. **#1023**: rg without `--no-config` — a user's RIPGREP_CONFIG_PATH (e.g. `--follow`) could change what the search fence sees; unprobed with rg installed.

## Open Issues Summary (agent-self, 15 open)
#1023 (fence residue: rg config/footer/find warnings), #1021 (stale gemini ids), #1018 (durable todos with due dates, from #682 design), #1016 (piped held text on other exits — Drop is cause-keyed), #1014 (/spawn write tap promised in #734), #1012 (75 price rows drift vs models.dev), #1011 (deny vs eval "$(...)"), #1009 (quote-wrapping residue: grep/watch/outline/docs/skill/lint), #988 (cancel → StopReason::Aborted audit), #982 (exit 0 on failure residue), #944 (phases with no usage record), #936 (50-verb near-miss residue), #902 (instruction files trust door), #879 (composite safe mode), #869 (/cd reloads: permissions allow, hooks, MCP, skills still not).

## Research Findings
- yopedia recall: prior notes exist ("Agent Changelog Delta Analysis", "Claude Code v2.1.166-183", "AI coding agents 2026 competitive landscape" July). Built on those; new note ingested today (job 71b4a483) covering CC 2.1.295-299 deltas and the Gemini id measurement.
- Claude Code is shipping at ~1 release/day (2.1.295 on Oct 8 → 2.1.299 on Oct 14); recent content is mostly enterprise/gateway/mods/Slack, plus the items in Capability Gaps. The coding-core deltas relevant to yoyo are small: idle compaction, file-mention mode, hook failure policy.
- **#1021 re-measured today (2026-10-15)** on https://ai.google.dev/gemini-api/docs/models: `gemini-3.0-pro` and `gemini-3.0-flash` still appear **0 times**. Present ids include gemini-2.5-pro/flash/flash-lite, gemini-2.0-flash, gemini-3-pro-preview, gemini-3-flash-preview, gemini-3.1-pro-preview, gemini-3.1-flash-lite, gemini-3.5-flash(-lite), gemini-3.6-flash, gemini-3.8-flash, gemini-4-argon. yoyo's google default `gemini-2.5-flash` (src/providers.rs:171) is still a live id, so the default is not broken — the known-model list is. Per Day-223 lesson, a fix should cover both halves: the ids made known AND what the unknown branch says (commands_info.rs:784 `model.contains("gemini-3")` fuzzy-matches any gemini-3.x to 1M — the Day-222 #1003 misroute class). Context windows/output limits for the new ids were NOT read from Google; do not type them from memory.

## Suggested direction for the planner (not task files)
- The trajectory says send the self-driven slot away from cd_config (5/9). Candidates outside it, each with a probe already run or a ticket comment naming the next step: **#1009** (`outline "a b.rs"` → `""a b.rs""` exit 0, confirmed today; remaining verbs grep/watch/outline/docs/skill/lint — do the Day-226 tool-comparison table first); **#1021** (providers; measurement reproduced above); **#1016** (prompt; the Day-228 reflection comment on #1016 already proposes the state-keyed predicate `closed && !open_text.is_empty()` — cite that comment as the source if selected, and decide the policy against yoagent's own `Held::Open` drop before coding).
- If a task fully fixes an issue, put `Fixes #N` in the task title; slices get `Part of #N`.
