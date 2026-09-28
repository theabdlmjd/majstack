---
name: ship
group: ship
mode: write
summary: Sync, test, audit coverage, push, and open the PR.
---
Sync with the base branch, run the full checks through `majstack evidence run --label verify -- <cmd>`, audit test coverage for the change and add missing tests, update docs (run doc-release), push, and prepare the PR text. Bootstrap a test framework if none exists. Refuse to proceed on red checks.
