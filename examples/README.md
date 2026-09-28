# Testing majstack

Three ways, fastest first.

## 1. Automated tests (no agent, no network)

The suite includes an end-to-end run driven by a mock provider: it plans, builds a DAG,
executes each task, verifies, reviews, commits, and completes. It also covers a blocked DAG
and parallel swarm execution.

```bash
cargo test --workspace
cargo test -p majstack-orchestration --test end_to_end -- --nocapture
```

If those pass, the engine is working.

## 2. Deterministic CLI run with the fake agent (no LLM, no network)

This drives the real binary end to end using a script as the "agent", so you can watch the
whole lifecycle and inspect the SQLite state, events, and commits.

```bash
mkdir demo && cd demo
git init -b main
git config user.email you@example.com
git config user.name you
cp /path/to/majstack/examples/fake-agent.ps1 .        # Windows
# cp /path/to/majstack/examples/fake-agent.sh .        # macOS/Linux
/path/to/majstack/target/release/majstack init
cp /path/to/majstack/examples/config.toml .majstack/config.toml
git add -A && git commit -m "init"

majstack run "create the demo files" --verify "cmd /C dir /b a.txt b.txt"
# macOS/Linux verify: --verify "test -f a.txt && test -f b.txt"

majstack status
majstack tasks
majstack logs
majstack graph
```

Expected: exit code 0, `a.txt` and `b.txt` created, task completed, a `feat: [...]` commit on a
`majstack/...` branch, and events in `majstack logs`.

For non-Windows, edit `.majstack/config.toml` to:

```toml
[agents.fake]
mode = "exec"
cmd = ["bash", "fake-agent.sh"]
instructions_file = "AGENTS.md"
```

## 3. Real run with a coding agent

Check what is installed, then point `run.agent` at it:

```bash
majstack doctor
majstack providers
# put agent = "claude" (or "codex" / "opencode") under [run] in .majstack/config.toml

majstack run "add a CSV export button to the reports page" --verify "npm test"
```

Use `--max-iterations 3` for a first, bounded run. `majstack status` / `majstack logs` / `majstack
evidence --labels verify` show what it did and whether the evidence is fresh.

## Other commands worth poking

```bash
majstack route "this endpoint is slow, investigate and fix it"
majstack tools
majstack workflows
majstack swarm --workers 3       # run ready tasks in parallel worktrees
majstack arena "implement a url shortener" -n 3
```
