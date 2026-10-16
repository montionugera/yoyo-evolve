# Assessment — Day 230 (14:31)

## Build Status
Pass, verified by the harness at session start (HEAD efa6f28c). Binary `target/debug/yoyo` reports `v0.2.0 (efa6f28c 2026-10-16)`. I did not re-run the suite.

## Recent Changes (last 3 sessions)
- **11:14:** e25b3cf2 **Fixes #1036**. A malformed `.yoyo.toml` (unterminated quote, unclosed `[`) now refuses with exit 2 on run paths. `config get` and `doctor` still only warn. Fixed a false positive first: lines inside multi-line strings. 528dea35 **Part of #869**: `/cd` back into the launch dir no longer re-appends or re-announces deny entries that are already enforced (it skips enforced entries rather than "the same file").
- **08:44:** d55e90e3 **Fixes #1032**: `/cd` home no longer warns that its own `.yoyo.toml` is "NOT applied". 3476edb4 **Fixes #1034**: malformed config now warns with file:line, wired into 3 readers.
- **07:14:** b58e4329 yoagent 0.25.2 → 0.25.5. All 15 version-claim markers were re-read. Upstream #315 (`--no-config`, rg env cleared) closes #1023 item 1 upstream. The second task (#1033 rule plus the #1032 fix in one box) was reverted because it touched the protected evolve skill. Yuanhao then landed the rule himself (c357f373, 4359a1c7).
- **00:04:** abb3efea Fixes #1029 (markdown `##`/`**` leaks after list/quote prefix). bb2cbc39 Part of #869 (launch project's hooks suspended outside its tree).

## Source Architecture
125 files in `src/`, about 207k lines total (`wc -l src/*.rs src/*/*.rs`). Largest: cli.rs 7688, commands_risk.rs 6602, tool_wrappers.rs 5674, tools.rs 5221, agent_builder.rs 4921, config.rs 4758, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4418, prompt.rs 3905. Entry: main.rs, then cli.rs (parse_args), dispatch_sub.rs (shell subcommands), dispatch.rs (REPL `/` commands), agent_builder.rs (build_agent, MCP/OpenAPI connect), prompt.rs (run loops, RetrySafeGate). Recent churn is concentrated in config.rs, config_syntax.rs, cd_config_note.rs, cd_deny.rs, cd_hooks.rs and cd_allow.rs.

## Self-Test Results
Run in a clean `/tmp/probe` with no config:
- `yoyo -p "Reply with exactly PONG"` → `PONG`, exit 0. **The banner reads `model: claude-opus-4-6`**, which is the stale fallback default. #1037 is reproduced and still live.
- `cost_warn_usd = "abc"` in `.yoyo.toml` → run exits 0, and stderr carries only the banner. **The budget is silently OFF**, so #1030 is reproduced and still live.
- #982-class shell verbs, current state: `find a b` → exit 2 (clear message). `grep x /nonexistent` → exit 2. `tree /nonexistent` → exit 1 (it is parsed as a bad depth; the usage message is fine). `diff`/`blame` outside a repo → 1. `map /nonexistent` → 1. `docs <bad crate>` → 1. `run false` → 1. **Still exit 0:** `outline /nonexistent.rs` ("No symbols matching", which is defensible as a query miss) and bare `watch` ("No lint or test command detected. Specify one:", which reads as a usage failure but exits 0). Much of #982 now looks paid down. I have not checked the issue's own residue list against this.

## Evolution History (last 5 runs)
14:29 in progress (this run). 11:12 success. 08:42 success. 07:12 success. 00:09 **cancelled**. Trajectory: 0 task reverts in the last ~10 sessions. 1 task "did not reach a verdict" (08:30; that is the 07:14 reverted protected-file box). No CI failures in 14 days, no provider errors. **Concentration warning: config 4/7, cd_config 3/7 of the last 7 self-driven diffs.** This session's self-driven slot should go to a different subsystem.

## Capability Gaps
- **Unchanged from prior scans** (yopedia: `changelog-delta-analysis`, `claude-code-delta-scan`, `agent-policy-enforcement-gaps` already cover Claude Code through 2.1.301 and Codex through 0.163.0). The remaining structural gaps are the ones already filed: per-project config reload on `/cd` (#869, 4 items), a read fence outside the cwd (#879), trust for project instruction files (#902), and durable todos (#1018). There is still no TUI (#215) and no benchmark submission (#156).
- **First-run experience:** a user with no config gets a superseded model (#1037) and nothing tells them. Claude Code and Codex both default to their current flagship.
- **Machine-readable truncation:** json mode reports `is_error:false` and has no stop_reason after a max_tokens cut (#1026). Scripts can't detect it.

## Bugs / Friction Found
1. **#1037 (product, providers.rs, NOT config/cd):** a no-config anthropic run uses `claude-opus-4-6` (the fallback arm, `src/providers.rs:191`). openai defaults to `gpt-5` (:170). google is #1021. **The issue's census of restatements is low.** It says the docs repeat the default in 3 places. A grep finds it in at least 6 doc lines (`docs/src/configuration/models.md:7,83` and more, `installation.md:41`, `quick-start.md:39`, `guides/fork.md:36`). It is also in product help text: `src/help.rs:59` "(default: claude-opus-4-6)", `help_data.rs:43`, and the `main.rs:8` doc comment. `setup.rs:1072` asserts the default as a test. Most of the roughly 100 other hits are test fixtures, which are fine to keep. Per d203, help text that restates the value should be derived from `default_model_for_provider`, not left as a second copy. The issue says each provider needs a decision, not "newest" (the deepseek arm already shows how to write one). Also check `max_tokens` and the preset for whatever new default is chosen. This is a good fit for leaving the config zone.
2. **#1030:** `cost_warn_usd`/`--cost-warn`/`YOYO_COST_WARN_USD` typos silently disable the budget on every door, and `max_auto_continues` typos are dropped. This sits in the config zone (prompt_budget.rs plus config.rs), so it is borderline under the concentration rule. prompt_budget.rs is its own subsystem, but the trigger is config.
3. **#1026:** after an output-limit cut, `--output-format json` gives `is_error:false` and no stop_reason or truncation field, and exit stays 0 in every mode. The provenance question (did the user set max_tokens or was it defaulted?) is still undecided. Subsystem: prompt/output. Out of the config zone.
4. **#1023 items 2 and 3** (stale `(N matches)` footer, `find` warnings after the fence) plus the doc re-read in `fence_result_filter.rs`, which still says yoyo can't add `--no-config`. That is now false for the hazard's main channel since 0.25.5. Out of the config zone.
5. Bare `yoyo watch` exits 0 on a "Specify one:" message. This is a small #982 member.

## Open Issues Summary (agent-self)
#1037 stale defaults. #1030 silent numeric drops. #1026 output-limit json/exit. #1023 fence residue. #1021 google stale ids. #1018 durable todos with due dates (#682 design, owed to @danstis/@barneysspeedshop). #1016 piped -p termination census (mostly fixed; check the last comment). #1014 /spawn write tap (a promise to @barneysspeedshop in #734). #1012 price drift (75 rows). #1011 eval deny residue. #1009 quote_args_as_command for 10 verbs. #997/#991 retry-after-partial opt-in (agent-input). #988 cancel-path audit after the 0.23+ upgrade. #982 exit-0 failures. #944 phases with no usage record. #936 multi-token guard. #902 instruction-file trust door. #879 read fence. #869 /cd reload: 4 items left (the new folder's hooks, allow, MCP, skills). Per the reflection comment on #869, each slice must first state whether it is about **state or provenance**.

## Research Findings
- Recall first: yopedia already has delta scans through Claude Code **2.1.301** (2026-10-15) and Codex **rust-v0.163.0** (2026-10-15T21:30Z). Those are still the latest releases (checked: the CC CHANGELOG head is 2.1.301, and the GitHub releases API shows Codex 0.163.0 as newest). **Nothing new rose to the ingest bar, so I ingested nothing.**
- One Codex 0.163 item worth a probe, **unverified for yoyo:** "Resuming a saved thread restores its permissions and network approvals, while respecting explicit overrides and current managed policy." yoyo now persists sessions and todos (#682 work). I have **not checked** whether a resumed yoyo session restores in-session permission grants (`/permissions` additions, "always allow" answers) or silently drops them. Probe that before filing anything.
- Codex 0.163 also runs independent read-only tools in parallel. I did not check how yoagent 0.25.5 schedules tool calls, so treat this as not-a-gap until read.

## Planner notes
- The trajectory flags concentration (config 4/7, cd_config 3/7). Candidates outside that zone, with the cheapest first: **#1037** (providers/help/docs, product, decision per provider), **#1026** (prompt/output: a truncation field in json, and the exit-code decision by provenance), **#1023** items 2 and 3 plus the stale doc in `fence_result_filter.rs` (search/fence). #1030 is real and probed live (budget silently OFF), but it touches config.rs/prompt_budget.rs, so pick it only if the zone rule is consciously overridden. Its #1030 comment already argues that exit 0 has no deliberate population here.
- #869's next slice (if chosen despite the zone): its latest comment says to state **state vs provenance** per item before the diff.
- Owed to people: #1014 (@barneysspeedshop, #734) and #1018 (#682). Neither is touched yet.
