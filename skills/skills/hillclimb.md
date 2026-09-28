---
name: hillclimb
group: build
mode: write
summary: Sustained scientific improvement of one metric against a target.
---
Define the metric, its measurement command, and the target. Loop: state a hypothesis, make the smallest change, measure, and keep it (one commit) only if the metric improves; otherwise revert. Log every attempt to `.majstack/decisions.tsv`. Stop at the target, at a plateau after five consecutive rejected hypotheses, or when told to.
