# Security policy

## Reporting a vulnerability

Open a private security advisory on the repository (Security → Advisories → New advisory), or
email the maintainer listed in the repository. Include a description, reproduction steps, and
affected version. Do not open a public issue for an unfixed vulnerability.

## Design

majstack treats everything outside the engine as untrusted:

- Model output, repository files, issue and PR text, CI logs, and web pages are **data**, never
  instructions. External text is wrapped in an untrusted envelope and injection markers are
  added; it is never elevated to system or tool policy.
- Project instructions in a repository cannot override the engine's permission or security
  policy.

## Permission model

Five levels, enforced centrally by the policy engine before any tool runs:

| Level | Meaning |
|---|---|
| read-only | observe only |
| safe | non-mutating commands |
| standard | normal file writes |
| autonomous | git operations, network reads, package managers |
| dangerous | destructive filesystem, force git, protected-branch push, merges, deploys |

Dangerous operations are denied unless explicitly enabled. Every tool declares a required
level. Every tool call is written to an audit table.

## Guards

- **Careful mode** classifies and blocks destructive shell patterns (force push, hard reset,
  recursive delete, drop/truncate, piped shell, `--no-verify`, protected-branch pushes).
- **Freeze** restricts edits to a set of directories; edits outside are reverted. Path
  traversal outside the workspace is rejected.
- **Untrusted envelope** flags prompt-injection patterns in external text.

## Secrets and data

- No telemetry, no analytics, no phone-home. The only network calls are the agent CLI you
  configure, an optional decision backend, and GitHub commands you invoke.
- All state is local to `.majstack/` in your repository.
- Log and evidence records pass through a redaction filter for common secret formats.
- An outbound action ledger is hash-chained and can be verified for tampering.

## Dependencies

Dependencies are pinned in `Cargo.lock`. Run `cargo audit` and `cargo deny` in CI before a
release. The CI workflow runs formatting, clippy with `-D warnings`, and the full test suite on
Linux and Windows.

## Supported versions

The latest released minor version receives security fixes.
