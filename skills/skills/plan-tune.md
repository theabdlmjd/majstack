---
name: plan-tune
group: plan
mode: write
summary: Tune which reviews and questions run, based on the user's past choices.
---
Read `.majstack/memory.jsonl` and `.majstack/reviews.jsonl`. Identify review steps the user always accepts or always overrides. Propose changes to `[pipeline]` and to review depth, and record confirmed preferences with `MAJSTACK_LEARN: PATTERN: ...`.
