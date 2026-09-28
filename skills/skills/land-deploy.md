---
name: land-deploy
group: ship
mode: write
summary: Merge, wait for CI and deploy, verify production health.
---
Confirm the ship gate is satisfied, merge the PR, wait for CI and the deploy, then verify production health with the configured canary. Report the result and be ready to roll back.
