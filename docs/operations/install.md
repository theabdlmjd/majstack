# Install and setup

## For users

1. Install the binary.

   Linux/macOS:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/theabdlmjd/majstack/main/install.sh | sh
   ```

   Windows PowerShell:

   ```powershell
   irm https://raw.githubusercontent.com/theabdlmjd/majstack/main/install.ps1 | iex
   ```

   The scripts fetch a prebuilt binary from GitHub Releases and fall back to
   `cargo install --git https://github.com/theabdlmjd/majstack majstack-cli majstack-daemon` when no release is
   available.

   From source, with Rust installed:

   ```bash
   cargo install --git https://github.com/theabdlmjd/majstack majstack-cli majstack-daemon
   ```

2. Set up a repository.

   ```bash
   cd your-repo
   majstack init
   majstack doctor
   ```

   `majstack init` creates `.majstack/`, seeds the bundled skills/workflows/principles, and writes a
   config whose `[run] agent` is the first coding-agent CLI it finds on PATH.

3. Run.

   ```bash
   majstack run "your goal" --verify "your test command"
   ```

## What you need

- A coding-agent CLI already installed and authenticated: `claude`, `codex`, or `opencode`.
  Any other CLI works too — add an `[agents.<name>]` entry with `cmd = [...]` in
  `.majstack/config.toml`.
- Optional: `gh` for pull requests and checks, and a Chrome/Edge/Chromium install for
  `majstack browse`.

## No environment variables are required

`majstack` reads `.majstack/config.toml`. It does not require secret env vars of its own; agent CLIs
use their own authentication. `GITHUB_TOKEN`/`GH_TOKEN` are read only if you use the REST
fallback for GitHub operations when `gh` is not installed.

## Layout of a configured repo

```
.majstack/
  config.toml        settings, agents, router overrides
  majstack.db           SQLite state (runs, tasks, evidence, events, ...)
  skills/            bundled and editable skills
  playbooks/         bundled and editable workflows
  principles/        bundled and editable principles
  logs/              per-call agent logs
  worktrees/         isolated task workspaces for swarm/arena
  shots/             browser screenshots from majstack browse
  STOP               created by majstack pause, removed by majstack resume
```

## Releases

Tag `vX.Y.Z` and push the tag. `.github/workflows/release.yml` builds `majstack` and `majstackd` for
Linux, macOS (x86_64 and arm64), and Windows, and attaches the archives to the GitHub release
using the exact asset names the install scripts expect (`majstack-<target>.tar.gz`,
`majstack-x86_64-pc-windows-msvc.zip`).
