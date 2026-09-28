---
name: plan
group: plan
mode: write
summary: Break a goal into small, ordered, independently verifiable tasks.
---
Turn the goal into the smallest ordered list of tasks. Each needs a title, acceptance criteria, likely files, a priority (lower runs first), dependencies by id, and `ui: true` when it changes what a user sees.
Prefer vertical slices that leave the repo working after every task. Put schema and shared types before code that uses them. Write tasks to `.majstack/tasks.json` as instructed. Do not write product code.
