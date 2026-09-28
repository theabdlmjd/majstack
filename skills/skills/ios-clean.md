---
name: ios-clean
group: ios
mode: write
summary: Remove debug bridges and test scaffolding before release.
---
Find and remove debug-only bridges, test hooks and logging left in the iOS target. Confirm the release build has none.
