---
name: perf
group: build
mode: write
summary: Trace measured slowness and improve it against a baseline.
---
Measure first: record a baseline with `majstack benchmark` or a profile. Find the actual hot path from evidence, form a hypothesis, change one thing, and re-measure. Keep the change only if the metric improves beyond noise. Report before and after numbers.
