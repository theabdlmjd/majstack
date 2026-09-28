---
name: migrate-callers
group: build
mode: write
summary: Migrate all callers and delete the legacy API together.
---
Find every caller of the legacy API, migrate them in one wave to the new one, delete the old API, and run the full checks. Write a codemod or script when there are more than a handful of call sites, and keep it as the reviewable artifact.
