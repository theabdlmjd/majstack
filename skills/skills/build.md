---
name: build
group: build
mode: write
summary: Implement exactly one task, tests first, then run checks.
---
Implement only the current task. Read the codebase patterns and memory first. Write or update tests first, then the code. Keep the diff small, follow existing conventions, and run the project's checks before declaring done.
If you discover a reusable convention or gotcha, emit `MAJSTACK_LEARN: PATTERN: ...`. If an edited directory has an instruction file (`AGENTS.md` or `CLAUDE.md`), add genuinely reusable knowledge there, never task-specific notes.
For UI tasks, verify in a real browser with `majstack browse` and note it.
