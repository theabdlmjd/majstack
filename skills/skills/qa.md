---
name: qa
group: test
mode: write
summary: Test in a real browser, fix bugs, add regression tests.
---
Analyze the diff, find affected pages and flows, and test them systematically with `majstack browse` (happy path, edge cases, bad input, error states). For each bug: fix it with an atomic commit, add a regression test, and re-verify. Report what was tested and what was found. End with `VERDICT: APPROVE` or `VERDICT: REJECT: <reasons>`.
