# majstack plan and status

An autonomous AI engineer: a goal goes in, verified results come out. The user does not pick
planners, coders, testers, reviewers, agents, models, or workflows — and does not need to code.

## Architecture

- Event-driven run state machine (not one giant loop).
- SQLite holds all durable state; the model context is disposable and rebuilt each iteration
  from a token budget.
- A real task DAG drives scheduling with dependency-aware, parallel execution.
- Data-driven routing chooses the work type and workflow from free text.
- Every tool, provider, and integration sits behind an interface; every meaningful action is an
  auditable event.

## Lifecycle

```
requested -> understanding -> investigating -> specifying -> reviewing_spec
-> planning -> reviewing_plan -> decomposing -> ready -> executing
-> verifying -> reviewing -> repairing -> reverifying -> completed
```

Suspended or terminal: paused, blocked, failed, cancelled, needs_human_input. The engine moves
backward when evidence invalidates an assumption.

## Persistent entities

projects, runs, goals, specifications, plans, tasks, task_dependencies, task_attempts, agents,
agent_runs, workflow_runs, workflow_steps, skills, skill_invocations, decisions, discoveries,
artifacts, evidence, verification_runs, verification_results, review_findings, failures,
recovery_actions, commits, branches, worktrees, provider_runs, model_runs, tool_calls,
browser_sessions, browser_artifacts, human_interventions, events, features, run_journal,
principle_invocations.

## Implemented

- Run state machine with persisted transitions and an event per transition.
- Feature entity with spec and plan artifacts and phase-skip resume.
- Planning, task parsing, DAG scheduling (priority, dependencies, atomic claims, roll-up).
- Fresh-context prompt assembly with token budget, stage-scoped principles, and memory recall.
- Verification (typecheck, lint, tests, build) with content fingerprints and evidence records.
- Independent review, review panels, and specialist roles.
- Failure classification, recovery planning, retries, escalation, and re-planning.
- Per-iteration run journal (model, duration, cost, files changed, notes).
- Decision trail and machine-readable, traceable principles.
- Git (branches, commits, worktrees, merges) and GitHub (issues, PRs, checks).
- Browser driver over the Chrome DevTools Protocol, with screenshots and console capture.
- Auditable tool registry, permissions, command guard, freeze, and untrusted-input handling.
- Parallel swarm, arena, and panels; a queue-processing daemon; typed decision backends
  (hosted or any OpenAI-compatible local model).
- CLI with human and JSON output.

## Remaining

- Design catalog and visual-parity depth.
- Diataxis documentation generation.
- Host lifecycle-hook installation.
- Canary and benchmark CLI wiring.
- Device and iOS QA.
- Chrome extension and external knowledge-base integrations.
