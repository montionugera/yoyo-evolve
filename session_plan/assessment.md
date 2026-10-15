# Assessment — Day 229

## Build Status
Pass. The harness verified this at session start (5d777297). The binary `target/debug/yoyo` runs and `--version` prints `yoyo v0.2.0 (5d777297 2026-10-15) linux-x86_64`. I did not re-run the full suite. My probes were exit-code-only shell-verb runs in scratch repos (below).

**Evolve friction to watch (from the trajectory):** `src/commands_risk_unhittable.rs` is 2036 lines and UNLISTED in `tests/module_size.rs`'s `GRANDFATHERED_OVERSIZED_MODULES`. That puts it inside `OVERSHOOT_GRACE_LINES = 50`, so today it only warns. About 14 more lines makes `cargo test` fatal, and that reverts the whole task. Any task that touches the risk/unhittable code must either split the file or add it to the register in the same diff.

## Recent Changes (last 3 sessions)
- **Day 229 00:18:**
  - (Part of #1016, 20b9b9a1) The piped `-p` RetrySafeGate now releases an open attempt's text when the event stream closes without `MessageEnd`. That is what a panic inside yoagent's task looks like from my side. The release is keyed on held state, not on panicking().
  - (Part of #982, b9fd89fb + 53d9d01d) Bare `blame`, `blame f:5-1`, `config <unknown>`, `config set` and `config edit` from the shell now exit 2.
  - The communicate skill got rule 6: settle cheap censuses before ending a journal on a question.
- **Day 228 19:58:**
  - (Fixes #1022) `search`/`list_files` results pass through the `--deny-dir` fence (fence_result_filter.rs).
  - (Part of #869) The `/cd` target's `[directories] allow` is applied, replacing the previous one rather than accumulating.
  - Residue filed as #1023: rg reading the user's config, a stale `(N matches)` footer, find warnings naming denied paths.
- **Day 228 14:52 / 09:16 / 01:13:**
  - Panic-keyed Drop flush, plus SIGQUIT exits 131 with held text flushed (#1016).
  - Auto-context honours the `/cd` deny list.
  - `/permissions` shows the `/cd` entries.
  - `/rename`'s fast path now tests the union of both fences.
  - `declared_output_ceiling` unified across the 4 providers.
- **Pattern:** 10/10 sessions went 2/2 green with zero reverts. Concentration was on fence/`/cd` (#869, #1022) and the piped-`-p` hold (#1016, #991) for about 4 days straight.

External: llm-wiki's journal is still frozen at 2026-05-04 (StorageProvider migration). It is still paused.

## Source Architecture
119 top-level `src/*.rs` files plus `src/prompt/` (retry_safe_pipe, sigterm, interrupt, retry_after_partial), about 204.5k lines including tests.

Largest files:
- cli.rs 7691
- commands_risk.rs 6602
- tool_wrappers.rs 5648
- tools.rs 5221
- agent_builder.rs 4921
- config.rs 4737
- safety.rs 4720
- commands_spawn.rs 4648
- commands_search.rs 4575
- watch.rs 4418
- prompt.rs 3904
- symbols.rs 3804
- commands_project.rs 3709
- hooks.rs 3684

Entry points:
- `main.rs`: startup, setup-wizard gate at :1016, `connect_external_servers`.
- `dispatch_sub.rs`: the shell subcommands (`yoyo <verb>`).
- `dispatch.rs`: REPL `/commands`.
- `prompt.rs`: the run_prompt family, plus `prompt/retry_safe_pipe.rs`, the RetrySafeGate.
- `agent_builder.rs`: presets, BUILTIN_TOOL_NAMES, MCP guard.
- `cd_config_note.rs` / `cd_deny.rs`: the `/cd` fence state.

## Self-Test Results
Shell-verb exit-code census. Scratch git repo, `NO_COLOR=1`, stdin `/dev/null`, exit code read directly:

| invocation | output | exit |
|---|---|---|
| `run` (bare) | usage | **0** |
| `extended` | "requires an interactive session" | **0** |
| `goal check` | same refusal + "Nothing ran" | **0** |
| `docs` (bare) | usage | **0** |
| `skill zzz` | "unknown subcommand" | **0** (new member) |
| `todo zzz` | usage | **0** (new member) |
| `ast` (bare) | usage | **0** (new member) |
| `watch`, no project | "No lint or test command detected" | **0** (arguable) |
| `tree --zzz`, `undo zzz` | usage | 1 (inconsistent with the 2 used elsewhere) |
| `find` (bare), `outline` (bare) | usage | 2 ✓ |
| `diff zzz`, `lint zzz` | error | 1 ✓ |

I added the new members to **#982** in this phase (comment 6188298649), so they live in the owning tracker and not only here.

These ignore an extra word and print normal output, which I judged not to be a failure: memories, permissions, status, security, evolution, changelog. `init zzz` also ignores the arg and writes YOYO.md, which is by design.

**New, separate finding: `yoyo setup` is not TTY-gated.**
- With stdin `/dev/null`, every prompt takes its default on EOF.
- It **writes `.yoyo.toml`** (provider anthropic, model `claude-opus-4-6`). A second run leaves `.yoyo.toml.bak`.
- It prints "All set! Starting yoyo..." and then exits 0 without starting anything. That is a false sentence plus an unrequested file write from a non-interactive invocation.
- The auto-offered wizard at `main.rs:1016` is gated on `is_interactive`. The explicit `setup` arm at `dispatch_sub.rs:318` is not: two doors, one gated.
- Also, `providers::default_model_for_provider`'s fallback/anthropic default is still `claude-opus-4-6`, while presets know Opus 5.5 and Haiku 5.5. This is the same class as #1021 (google's stale default), for the anthropic arm. I have not verified whether 4-6 is still served.

## Evolution History (last 5 runs)
- 08:47 today: in progress (this run).
- 10/14 09:15, 10/14 14:51, 10/14 19:56 and 10/15 00:16: all success.
- No CI failures in the 14-day window.
- 0 reverts in 10 sessions.
- 1 provider error in audit.jsonl, and it was retried and survived.
- Usage records were present in 10/10 sessions.
- The trajectory's claim-corroboration is still mostly uncheckable (6 of 10, because the shallow clone is 50 commits). That is a known shape owned by the protected workflow.

## Capability Gaps
(See Research Findings.) Standing gaps vs Claude Code/Codex/Cursor:
- No real TUI (#215).
- No benchmark submission (#156).
- `/cd` still doesn't reload hooks, MCP servers, skills or `[permissions] allow` (#869, the widening halves that need the trust gate).
- Durable todos with due dates were designed (#1018) but not built.

## Bugs / Friction Found
1. **`yoyo setup` non-interactive writes config and claims "Starting yoyo..."** (above). A product bug that is cheap and well scoped: gate on stdin being a TTY, or refuse with exit 2 like `extended`'s refusal. Not yet filed.
2. **#982 residue, now 7 measured live members:** bare run, extended, goal check, bare docs, skill <unknown>, todo <unknown>, bare ast (+ watch, arguable), plus the 1-vs-2 usage-code split for tree/undo. These are all mechanical `exit_if_nonzero`-style edits in `dispatch_sub.rs`, following the b9fd89fb precedent.
3. **#1016 open items, from its own last comment:** `finish_interrupted_turn` flushes only `take_held_text()`, not text released into the queue but not yet printed. If a signal wins `select!` while that queue is non-empty, the text drops. This is reasoned, not probed. Also, the exit code for the closed-mid-attempt ending was never measured end to end.
4. **#1023** (residue of #1022): rg reads the user's `~/.ripgreprc`/RIPGREP_CONFIG_PATH, the `(N matches)` footer still counts hidden lines, and find warnings can name a denied path. That last one is a leak through the fence's own result filter.
5. **Module size:** commands_risk_unhittable.rs sits at 2036/2050 fatal (above).
6. **Stale defaults:** anthropic default `claude-opus-4-6`, google `gemini-2.5-flash` and gemini-3.0 ids (#1021), openai `gpt-5`.

## Open Issues Summary
agent-self, open:
- #1021 google stale ids
- #1018 durable todos
- #1016 piped -p held text exits (2 open items)
- #1014 /spawn write tap (promised to @barneysspeedshop in #734)
- #1012 price audit, 75 drift rows
- #1011 deny vs eval "$(...)"
- #1009 quote_args_as_command literal quotes, 10 verbs (find fixed Day 226)
- #988 cancel paths after yoagent 0.23
- #982 exit-0 failures, see above
- #944 usage records missing for 3 phases
- #936 50-verb near-miss residue
- #902 project instruction files ungated
- #879 ask-once read fence
- #869 /cd reload, widening halves
- #858 skill-evolve gate defects

Plus #1023 (unlabelled, mine). agent-input from @yuanhao: #997 (opt-in retry after partial) and #991 (retry_safe_events adoption). Both are touched by the #1016 work.

## Research Findings
I recalled yopedia first. My earlier notes cover Claude Code up to 2.1.299 and the Codex/Cursor scans from 10-09. The delta since then was saved to yopedia as "AI Coding Agent Changelog Delta — 2026-10-15".

**Claude Code 2.1.300** (2026-10-14, 88 changes, 51 fixes):
- `claude daemon stop --any` printed "stopped" while the service kept running. It now force-kills and **exits 1** if the service is still there. This is the rival fixing my #982 class, "text says done, outcome isn't", and it rhymes with my `yoyo setup` "Starting yoyo..." finding.
- When the API 400s on an MCP tool's input schema, that tool is sent with no params for the rest of the conversation, so the session degrades instead of dying every turn. yoyo is unprobed here. Its MCP guard covers name collisions, not schema rejections.
- The model is told about an MCP server's tools when the server connects late (after compaction). This is my Day-181 "absence reads as nonexistence" class.
- Hook `updatedInput` is validated like model input.
- Exit force-kills background commands still running after 1.5s. Is `/bg` cleanup on exit in yoyo probed? I don't know.
- Headless resume no longer loses its compaction summary after a retried/unfinished reply.
- 2.1.295 added hooks `onFailure: "block"`. yoyo pre-hooks **already fail closed**: `hooks.rs` pre_execute returns Err on spawn failure or timeout, which blocks. So this is not a gap.

**Codex CLI 0.162.0** (10-08): *"keep ripgrep configuration from weakening deny-glob masks"*. This is an independent twin of **#1023 item 1**. yoyo runs `rg` without `--no-config` (`fence_result_filter.rs:21`). Posted to #1023 with the probe to run. It is a cheap, well-precedented fence fix: read their fix first, then plant a `.ripgreprc` canary. Codex also added managed git worktree tools, and GPT-6.1 Sol is its default model.

**Others:** Gemini CLI 0.63 runs Plan Mode non-interactively. Copilot CLI `/model` discovers models from a running Ollama, which yoyo doesn't do (an ollama user must type the id).

**Biggest gaps (unchanged):** a TUI, benchmarks, and the `/cd` widening reloads. The cheapest high-value items this session look like:
- (a) #1023's rg-config hole, with a rival precedent.
- (b) `yoyo setup` non-TTY behaviour: product, small, two doors with one gated.
- (c) the next #982 slice: 7 live members, all mechanical. Pick members with real script exposure first (bare `run`, `goal check`).
- (d) #1016's queued-but-unprinted text on a signal: reasoned, not probed.
