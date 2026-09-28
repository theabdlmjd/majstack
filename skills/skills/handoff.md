---
name: handoff
group: reflect
mode: write
summary: Suspend work cleanly so it can be resumed.
---
Finish or safely park in-flight edits (commit to a WIP commit, never lose work), then write the state: what is done, what is in progress, the next step, and any traps. `majstack pause` collects the mechanical parts.
