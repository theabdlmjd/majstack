---
name: setup-deploy
group: ship
mode: write
summary: One-time deploy configuration.
---
Detect the platform, production URL, health endpoint, and deploy command. Write them into `.majstack/config.toml` under `[release]` and confirm each by running it in dry-run mode where possible.
