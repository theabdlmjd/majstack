# Current system audit

The existing system is a Python package `majstack` (version 2.0.0) at `majstack/` with CLI entry
point `majstack.cli:main`. It is a working v2 that already combines the three source capability
sets in a single-process, file-backed form. This audit records what to preserve, what is weak,
and what is missing relative to the target architecture. Nothing is deleted merely to ease a
rewrite; every behavior below is preserved or intentionally superseded.

## Inventory of existing modules

| Module | Responsibility | Disposition in Rust rebuild |
|---|---|---|
| `cli.py` | argparse CLI, ~40 subcommands, template sync, instruction export | Reimplemented in `majstack-cli` (clap) |
| `state.py` | `.majstack/` layout, JSON task store, progress log, snapshots, archive, claims | Superseded by `majstack-state` (SQLite) + `majstack-git` snapshots |
| `engine.py` | agent selection by routing, agent call, learnings parse, guard enforcement | `majstack-providers` + `majstack-core::sigils` + `majstack-policy` |
| `loop.py` | autonomous loop: plan, run tasks, verify, review, retry, commit, finish | `majstack-orchestration` state machine + scheduler |
| `workflows.py` | skill runner, playbook runner, auto-router, panel, arena, swarm, gate, PR, land, pause/pickup, host install, stop hook | `majstack-workflow`, `majstack-orchestration`, `majstack-github` |
| `skills.py` | skill/playbook loading, keyword router, principles text | `majstack-skills` |
| `adapters.py` | agent spawn (exec/handoff), model args | `majstack-providers` |
| `browse.py` | Playwright browse, HTTP fallback, login state | `majstack-browser` |
| `ops.py` | fingerprint, egress ledger, evidence, readiness, untrusted envelope, decisions, board, retro, canary, benchmark, health, graph, suggest, worktree clean | `majstack-evidence`, `majstack-verification`, `majstack-policy`, `majstack-telemetry`, `majstack-git` |
| `guard.py` | destructive command patterns, freeze path policy, hooks, post-call revert | `majstack-policy` + `majstack-tools` shell guard |
| `git.py` | git subprocess wrappers, worktrees, merge, diff | `majstack-git` |
| `memory.py` | JSONL memory, token search, majstackt | `majstack-memory` (SQLite + FTS) |
| `prompts.py` | prompt composition with principles, guards, memory, task, progress | `majstack-orchestration` context builder |
| `templates/` | 85 skills, 36 playbooks, 23 principles, config.toml | `skills/` data + `majstack-skills` registry |
| `tests/` | 18 tests incl. loop, arena, swarm, freeze, evidence, ledger | Ported to `tests/` as Rust integration tests |

## Preserve (high value, already correct)

1. **Fresh-context loop with on-disk state** (`loop.py`): the core primitive. Preserve
   behavior, move persistence to SQLite.
2. **Sigil-based result protocol**: `MAJSTACK_DONE`, `MAJSTACK_BLOCKED`, `MAJSTACK_LEARN: PATTERN:`,
   `VERDICT: APPROVE/REJECT`, `WINNER: n`. Generalized in `majstack-core::sigils`.
3. **Evidence binding to content fingerprint** (`ops.fingerprint`): strongest existing idea.
   Review/evidence freshness tied to exact tree content. Atomic write-tree approach preserved
   in `majstack-evidence`.
4. **Tamper-evident egress ledger**: hash-chained records. Preserved verbatim in intent.
5. **Permission guard**: destructive pattern list, freeze path allow-list, post-call revert,
   host hook installation. Mapped to `majstack-policy` with explicit permission levels.
6. **Skill/playbook loaders**: YAML frontmatter skills, TOML playbooks, keyword router,
   step tokens (`@loop`, `@swarm`, `@arena`, `@babysit`, `@interrogate`, `@gate`, `@land`).
7. **Memory token search** and pattern promotion.
8. **Arena** (N worktree contestants + judge + merge), **swarm** (parallel worktrees,
   ordered merge), **interrogate panel** (lenses across agents).
9. **PR open / babysit / land gate**.
10. **Untrusted-text envelope** for issue/PR/CI text.
11. **Evidence/readiness/gate** semantics.
12. **Health detection** across npm/pytest/cargo/go.
13. **Retro stats, benchmark with previous comparison, canary loop**.
14. **Task DAG with priority, depends, ui flag; ready selection; claims**.
15. **Pause/handoff/pickup** and snapshots/rollback.
16. **Template sync with customization preservation** (`upgrade`).
17. **Host skill installation** for claude/codex/opencode/cursor/factory/kiro.
18. **Instruction export** into `CLAUDE.md`/`AGENTS.md` between managed markers.
19. **Stop hook** that keeps an agent working until verify passes (yields after 3).

## Weak abstractions / technical debt to fix

1. **JSON file state is not transactional** and races under parallelism. Claims are
   lock-files; the task file is rewritten whole. Superseded by SQLite transactions and rows.
2. **Routing is keyword scoring** (`skills.route`) and ad-hoc agent routing. Replace with the
   data-driven router (work type, investigation need, evidence, capabilities, parallelism,
   provider/model, verification, retry). No giant if/else chain.
3. **The loop mixes concerns**: planning, execution, verification, review, commit, finish all
   in `loop.py`. Replace with the explicit event-driven state machine.
4. **No real task graph object**: dependencies are raw lists; cycle detection is absent (the
   test even depends on a nonexistent task). Add validated DAG with hierarchy and rollup.
5. **Fresh context is implicit**: each `call` builds a prompt, but there is no explicit
   iteration context object, token budget, or context digest.
6. **Model strategy is absent**: agent/model is chosen once by routing; no escalation.
7. **Failure handling is a boolean retry** with a "debug" skill. No classification, no
   strategy adaptation, no escalation, no re-plan path.
8. **Verification is a single shell command plus optional UI/review strings**. No structured
   result records, durations, artifacts, or multi-check registry.
9. **Review findings are free text**; no severity/category/file/line/blocking fields.
10. **Playbooks are hardcoded step lists** interpreted by the orchestrator. Replace with a
    workflow registry with triggers, phases, transitions, capabilities, failure strategy.
11. **Skills are prose only**. Add metadata, contracts (output/verification), prerequisites,
    failure conditions, composition, cycle prevention.
12. **No provider interface**: `Agent` fuses process spawn, handoff prompting, and model args.
    Replace with a session-oriented provider trait (start, send, stream, cancel, resume,
    usage, identity, permissions).
13. **No unified tool registry / audit**: filesystem, shell, git, browser are called directly.
14. **No event stream**: progress is a Markdown log. Add typed events persisted and replayable.
15. **No telemetry/context accounting**.
16. **Security is partial**: guard patterns exist but permissions are not centrally enforced
    per tool; `.majstack` is exempt from freeze.
17. **Browser is a thin snapshot** (text/links/console/failed requests + screenshot). No click,
    fill, keyboard, wait, evaluate, viewport, accessibility, network capture, or assertions.
18. **Placeholder product name** `YOURNAME` throughout README/LICENSE.

## Missing functionality (relative to target)

The following do not exist in the Python system and must be built: event-driven state machine
with backward transitions; run/goal/specification/plan/task_attempt/workflow_run/provider_run/
model_run/tool_call/browser_session/human_intervention/failure/recovery_action entities;
SQLite migrations and crash recovery; agent_runs; decisions/discoveries as first-class rows;
the autonomous router as a data-driven layer; workflow registry with nested workflows; skill
contracts and composition; provider session interface with streaming and cancellation;
unified tool registry with audit; verification result registry; review finding registry;
failure classification and recovery engine with strategy adaptation and escalation;
permission levels enforced centrally; parallel review panels with evidence merge; worktree
isolation for parallel tasks; browser interactivity and evidence; GitHub checks/actions/
labels/milestones; daemon mode for long-running/overnight work; machine-readable capability
parity gate in CI.

## Duplication and unnecessary complexity

- Agent spawning logic duplicated between `engine.call` and `workflows.model_benchmark`.
- Verification command resolution duplicated (`engine.verify_cmd`, `loop`, `workflows`).
- `.majstack` path handling is stringly and split across modules.
- `state.py` mixes layout, persistence, archiving, snapshots, and claims.
- `ops.py` is a 292-line grab bag of ten unrelated concerns; split across crates.
- Both `progress.md` and `memory.jsonl` persist patterns (double bookkeeping).

## Compatibility

The existing `.majstack/` directory layout (skills, playbooks, principles, config.toml) is
retained as the on-disk authoring format so all 85 skills, 36 playbooks, and 23 principles
remain usable. Runtime state moves to a SQLite database. A migration command imports legacy
`tasks.json`, `progress.md`, `memory.jsonl`, `evidence.jsonl`, `reviews.jsonl`, `egress.jsonl`,
and `decisions.tsv`.
