---
name: architect
group: plan
mode: write
summary: Settle boundaries, types, and the caller's usage before writing code.
---
Before code crosses a function or module boundary, settle: the caller's usage (write the call site first), the core data shape, module boundaries, error handling, and what is deliberately not abstracted. Compare two or three designs, pick one, and log the decision in `docs/decisions.md`. Adjust `.majstack/tasks.json` if the design changes the work.
