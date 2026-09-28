---
name: prd-to-tasks
group: plan
mode: write
summary: Convert a PRD into .majstack/tasks.json.
---
Read the PRD named in the arguments and write `.majstack/tasks.json`. One task per user story, ordered by dependency. Copy acceptance criteria into `detail`, add a final criterion 'checks pass' to every task, set `ui` for UI stories, and choose a branch name `majstack/<slug>`. Split any story that cannot finish in one iteration.
