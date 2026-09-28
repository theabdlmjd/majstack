---
name: refactor
group: build
mode: write
summary: Behavior-preserving change to structure.
---
Establish characterization tests first so behavior is pinned. Change structure in small steps, running tests after each. Migrate all callers and delete the old API in the same change; do not leave compatibility layers. Prove behavior is unchanged with the tests, and show the net deleted lines.
