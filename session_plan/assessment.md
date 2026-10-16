# Assessment — Day 230 (07:14)

## Build Status
Pass — verified by the harness at session start on the **0.25.2 lock**. Binary `yoyo v0.2.0 (6b64946a)`: `-p "Reply with exactly: PONG"` → `PONG`, exit 0; `yoyo grep zz9qq /nonexistent` → error + exit 2 (the #982 grep fix holds).

**Headline finding: the harness's new Step 1a (dependency update, creator commit 6b64946a, landed 07:07 today) tried yoagent 0.25.2 → 0.25.5 at session start, the update was rejected, and Cargo.lock went back to 0.25.2.** I reproduced it myself (ran `cargo update -p <each direct dep>`, then `cargo build`, then `cargo test --bin yoyo` and `cargo test --no-fail-fast --test '*'`, and restored the lock afterwards; the tree is clean):
- `cargo build`: OK.
- `cargo test --bin yoyo`: **6244 passed, 0 failed**.
- Integration tests: **exactly one failure**, `tests/doc_version_claims.rs::every_dependency_version_claim_names_the_pinned_yoagent_version`. It is the designed tripwire: **15 `yoagent-version-claim: 0.25.2` markers**, and each one says "re-read the paragraph above this marker":
  - CLAUDE.md:10; ARCHITECTURE.md:263, 269, 508, 532
  - src/format/cost.rs:2497, 2540; src/gasp.rs:32, 1013; src/prompt.rs:2992
  - src/tool_wrappers.rs:1481, 1513, 5308; src/tool_wrappers_cancel_tests.rs:31; src/tools.rs:1955
- Cost while this stays red: every session's Step 1a pays an extra build and a full `cargo test` with the new lock, fails on this tripwire, and restores the old lock. Then Step 1 rebuilds against the old lock. The harness will not land the upgrade alone, by design, so a task has to land it.
- Other direct deps in the same update: serial_test 3.4→3.5, tempfile 3.26→3.27, tokio-util, socket2, plus windows-sys removals. All tests pass with them.

## Recent Changes (last 3 sessions)
- **Day 230 00:04**: Fixes #1029 (streamed markdown leaked `##`/`**` after list/quote prefixes; split-delta output now matches whole-line output, and 4 old tests that pinned the leak were changed). Part of #869: the launch project's own hooks pause after `/cd` leaves its tree, `/hooks` shows them `suspended`, and they come back on `/cd` back. **Side effect:** the commit body said `Seen, not fixed: #1032.`, GitHub read `fixed: #1032` as a closing keyword, and #1032 was closed. The creator reopened it and filed **#1033** (the rule must go into `skills/evolve/SKILL.md`).
- **Day 229 19:41**: Fixes #1028 (bad numeric values in .yoyo.toml now warn); interactive-only refusals (`extended`, `goal check`, `lint fix`) exit 2. Residue filed as #1030.
- **Day 229 14:40**: a reply cut off at the output limit gets a stderr note, and exit stays 0 by decision (residue #1026). Part of #869: the launch `[permissions] allow` is suspended after `/cd`, inside the authority.
- **Creator, today**: `21a75910` loads skills from any dependency crate (yoagent 0.25.3+ ships `skills/yoagent/SKILL.md`, so the skill only loads once we're on 0.25.5). `6b64946a` adds Step 1a, semver dependency updates. Both are in evolve.sh (protected).
- llm-wiki side project: still paused mid-migration (per journal).

## Source Architecture
124 files in src/, ~206K lines incl. tests. Largest: cli.rs 7686, commands_risk.rs 6602, tool_wrappers.rs 5674, tools.rs 5221, agent_builder.rs 4921, config.rs 4742, safety.rs 4720, commands_spawn.rs 4648, commands_search.rs 4575, watch.rs 4418, prompt.rs 3905, symbols.rs 3804, commands_project.rs 3709, hooks.rs 3695. Entry: main.rs → cli::parse_args → agent_builder (build_agent, connect_external_servers) → repl / prompt::run_prompt*; dispatch.rs (REPL /commands), dispatch_sub.rs (shell subcommands).
**Size gate (from trajectory):** commands_risk_unhittable.rs is 2036 lines, unlisted, and 14 more lines make the gate fatal. format/mod.rs is 2874 vs its recorded 2788, also 14 lines from fatal. Any task touching either must split it or update the register.

## Self-Test Results
- `-p` PONG works, exit 0, and the auto-watch line prints (evolve-repo config).
- `yoyo grep` on a nonexistent path: exit 2. The message is git's raw `fatal: ambiguous argument` text, which is legible enough.
- Upgrade probe: see Build Status. Only the version-claim tripwire is red under 0.25.5.

## Evolution History (last 5 runs)
07:12 in progress (this run). 00:09 **cancelled** (workflow_dispatch, 7 min after the 00:02 scheduled run started; most likely superseded or a manual cancel, I didn't dig further). 00:02, 19:40 and 14:38 success. Trajectory: 10/10 sessions at 2/2 tasks, 0 reverts, no CI failures in 14 days, no provider errors.

## Capability Gaps
- **yoagent 0.25.5 upstream fixes I'm not getting** because of the lock: `search` passes `--no-config` and clears `RIPGREP_CONFIG_PATH`/`GREP_OPTIONS` (yoagent #315, *reported by yoyo*: this is #1023 item 1 fixed at the source). `search` finds `rg` without `which` and reports `details.engine`. **A search that ends abnormally is no longer "No matches"** (the same class as #982/#1023). 0.25.4: ToolGate judges large calls whole (yoyo doesn't seem to use ToolGate; a grep of src finds none). And the shipped yoagent skill.
- After the upgrade, yoyo's own Day-229 RIPGREP_CONFIG_PATH defence (src/fence_result_filter.rs:109, :326, withheld-line note) becomes defence in depth. Its note text ("a ripgrep config via RIPGREP_CONFIG_PATH can reshape output") is a claim about the dependency that 0.25.5 makes false for the search tool. Re-read it in the upgrade task (d222/d211: retire the workaround or rewrite it in the past tense).
- Longer-standing gaps vs Claude Code: no TUI (#215), the read fence is not ask-once (#879), project instruction files have no trust gate (#902), /cd reloads only part of the config (#869: new dir's hooks, its allow list, MCP, skills).

## Bugs / Friction Found
1. **The yoagent 0.25.5 upgrade is blocked only by 15 version-claim markers.** Highest leverage: it unblocks Step 1a every session (saves a wasted full suite plus a rebuild each time), brings in the upstream search fixes, and lets the yoagent skill load. Each marker needs its paragraph re-read against 0.25.5 source (`~/.cargo/registry/src/*/yoagent-0.25.5/`); the CHANGELOG for 0.25.3–0.25.5 is the diff to check them against. The final commit must include Cargo.lock at 0.25.5 (`cargo update -p yoagent` plus the other direct deps, or let it be yoagent-only). **Check the closing-keyword rule (#1033) on its commit message.**
2. **#1033 (creator-filed, the done-when is explicit)**: add the closing-keyword-in-BODY rule plus the grep check to `skills/evolve/SKILL.md`, next to the Day-228 title lesson. Small, not protected. It also protects every other task this session, so do it first or alongside.
3. **#1032**: `/cd` back into the launch dir prints a false "NOT applied" warning (cd_config_note never compares the target's .yoyo.toml with LOADED_CONFIG_PATH). Probed and specified in the issue. Small and product-facing.
4. **#1030**: `cost_warn_usd`/`--cost-warn abc` silently turns the budget OFF on every door; `max_auto_continues` is dropped silently. Specified in the issue, mirrors #1028.
5. **New, probed Day 230, not filed: malformed `.yoyo.toml` syntax loads silently.** In /tmp/tomlprobe, a file containing `model = "claude-haiku-5-5` (unterminated string) plus an unclosed `[permissions` header gives `yoyo config get model` → `"claude-haiku-5-5` (exit 0). `-p` then sends the id *with the stray leading quote* and gets a 404, and nothing ever says the file is malformed. `parse_config_file` (src/config.rs:1984) is a hand-rolled line parser: when the quotes don't match it keeps the raw value (`_ => value.to_string()`), and it has no channel for errors. #1028/#1030 cover bad numeric *values*, not syntax. Claude Code 2.1.29x fixed the same thing: "`claude -p` saying nothing when a settings file fails validation". A minimal shape: a warning naming the file:line for an unterminated quote / unclosed `[` header, through `sanitize_for_display`, while keeping today's lenient value (the near-miss guard: valid files stay byte-silent). This isn't on the tracker yet; file it or make it a task.
6. #1026 (json/stream-json print no output-limit note), #1009 (nine verbs still eat quotes), #982 residue (4 items), #1023 residue (stale footer, find warnings).

## Open Issues Summary
agent-self (19 open): #1033 commit-body keyword rule (creator); #1032 false NOT-applied warning; #1030 silent bad cost/continue values; #1026 output-limit note in json modes; #1023 fence residue; #1021 stale gemini ids; #1018 durable todos with due dates (from #682); #1016 piped -p held text on other exits; #1014 /spawn write tap (promised in #734); #1012 price audit, 75 drifting rows; #1011 eval "$(...)" deny bypass; #1009 quote-eating verbs; #988 cancel paths after Aborted; #982 exit-0 failures; #944 phases with no usage record; #936 50-verb near-miss residue; #902 instruction-file trust door; #879 read fence; #869 /cd config reload (4 parts left). agent-input: #997, #991 (retry after partial; #991 partly done Day 226).

## Research Findings
(see below; updated after research)
ut-limit silent-cut item reappears (non-streaming fallback). yoyo did its version Day 229, and the residue is #1026.
- **yoagent 0.25.5 (released today)** fixes in the dependency what yoyo reported (#315 → yoyo #1023 item 1), and adds "abnormal search end is never No matches". Landing it is the single highest-leverage task: see Build Status / Bug 1.

## Suggested priorities (for the planner; my opinion)
1. yoagent 0.25.5 upgrade: re-read the 15 markers, update them, commit the lock. Title it "Part of …" or no keyword. Re-read fence_result_filter's RIPGREP note against 0.25.5. Check the #1023 body before claiming any of it fixed (d228: read the issue body for scope words).
2. #1033: the closing-keyword-in-body rule in skills/evolve/SKILL.md. Tiny, creator-asked, explicit done-when. It can ride with any task, and the planner should apply its grep to every commit message this session.
3. #1032 (false NOT-applied warning on /cd home), #1030 (silent cost budget off), or malformed-TOML warning (Bug 5): each is small and product-facing.
