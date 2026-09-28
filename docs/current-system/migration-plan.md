# Migration plan

## Principle
Port behaviors, not files. Every preserved behavior from `audit.md` lands in a named Rust
crate with a test. The legacy `.majstack/` authoring format (skills, playbooks, principles,
config.toml) is read directly so no content is lost; runtime state moves to SQLite through a
migration command.

## Target workspace

```
Cargo.toml                     workspace
crates/
  majstack-core/                  ids, errors, result, clock, slug, statuses, sigils, events
  majstack-config/                config load, schema validation, paths, defaults
  majstack-logging/               tracing setup, redaction filter
  majstack-telemetry/             context accounting, usage, metrics
  majstack-state/                 SQLite, migrations, repositories, recovery, archive
  majstack-events/                event bus, persistence, replay
  majstack-graph/                 DAG, hierarchy, scheduling, rollup, cycle detection
  majstack-policy/                permission levels, guards, untrusted envelope, freeze
  majstack-memory/                knowledge, journal, context digest, budgeted recall
  majstack-evidence/              fingerprints, evidence, reviews, decisions, egress ledger
  majstack-providers/             provider trait, CLI/API adapters, model strategy
  majstack-tools/                 tool registry, fs, shell, http, search, runner tools
  majstack-execution/             workspace, worktrees, process, sandbox, isolation
  majstack-git/                   git ops, worktrees, commits, branches, diffs
  majstack-github/                repos, issues, PRs, reviews, checks, actions, labels
  majstack-browser/               chrome CDP driver, command registry, artifacts, QA
  majstack-verification/          verification + review engines, findings, panels
  majstack-skills/                skill registry, principles, contracts, composition, authoring
  majstack-workflow/              workflow registry, router, playbooks, nested workflows
  majstack-agents/                specialist roles and team selection
  majstack-recovery/              failure classification, recovery, strategy adaptation
  majstack-orchestration/         run state machine, scheduler, fresh-context executor
  majstack-testing/               test harness utilities, eval harness
apps/
  majstack-cli/                   bin: majstack
  majstack-daemon/                bin: majstackd
skills/                        unified capability content (ported from templates)
tests/                         unit, integration, workflow, provider, browser, recovery, e2e, parity
docs/                          architecture, workflows, skills, providers, security, operations
```

## Order of work

1. Workspace scaffold and `majstack-core` with all enums/sigils/events. Compile green.
2. `majstack-config`, `majstack-logging`, `majstack-telemetry`.
3. `majstack-state` with full schema (all 32 entities), migrations, repositories, recovery.
4. `majstack-events` bus with persistence and replay.
5. `majstack-graph` DAG with unblock, rollup, cycle detection, hierarchical tasks.
6. `majstack-policy` permission levels, shell guard, freeze, untrusted envelope.
7. `majstack-memory` journal + knowledge + FTS + context digest.
8. `majstack-evidence` fingerprint, evidence, reviews, decisions, egress ledger.
9. `majstack-providers` trait + CLI adapters (claude/codex/opencode/cursor/custom) + model strategy.
10. `majstack-tools` registry + fs/shell/git/http/search/test/build/lint/typecheck/diff/patch.
11. `majstack-execution` workspaces and worktree isolation.
12. `majstack-git` operations.
13. `majstack-verification` runner registry + review engine + findings + panels.
14. `majstack-skills` registry seeded from `skills/`, principles, contracts, composition.
15. `majstack-workflow` registry + router + ported playbooks.
16. `majstack-agents` specialist roles.
17. `majstack-recovery` classification + adaptation.
18. `majstack-orchestration` state machine + scheduler + fresh-context executor.
19. `majstack-browser` CDP driver + QA workflows.
20. `majstack-github` issues/PRs/checks.
21. `apps/majstack-cli` and `apps/majstack-daemon`.
22. `majstack-testing`, full test suite, parity gate, docs, CI, notices.

## Data migration

`majstack migrate` reads legacy files and writes rows:
- `tasks.json` -> `goals`, `plans`, `tasks`, `task_dependencies`.
- `progress.md` -> `runs` summary + `events`.
- `memory.jsonl` -> `discoveries` (patterns -> knowledge with `pattern` tag).
- `evidence.jsonl` -> `evidence` + `verification_runs`/`verification_results`.
- `reviews.jsonl` -> `review_findings`.
- `egress.jsonl` -> `events` (egress) preserving the chain for verification.
- `decisions.tsv` -> `decisions`.
- `.majstack/archive/*` -> archived runs.

## CLI compatibility

Legacy command names are preserved where they remain meaningful (`init`, `run`, `status`,
`do`, `auto`, `playbook`, `skills`, `agents`, `evidence`, `gate`, `pause`, `pickup`, `pr`,
`land`, `memory`, `browse`, `retro`, `canary`, `benchmark`, `health`, `graph`, `decide`).
The target command set (`agent init/run/status/inspect/pause/resume/cancel/task/workflow/
skill/provider/doctor/verify/review/logs/evidence/graph`) is added alongside, with
`--json` machine output on every command.

## Parity gate

`tests/parity.rs` loads `docs/source-research/capability-inventory.json` and asserts every
entry has `implementation_status` of `native`, `mapped`, or `incompatible` with a reason, and
that no entry is `pending` at release. CI fails if an entry disappears or regresses.
