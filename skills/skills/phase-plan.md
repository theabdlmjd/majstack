---
name: phase-plan
group: plan
mode: write
summary: Plan multi-phase or stacked work where each phase is independently shippable.
---
Split the work into phases. Each phase must build, pass verification, and be reviewable on its own, and later phases only depend on earlier ones. Show the dependency graph first, define the checkpoint for each phase, and note where parallel work is safe. Write the phases as tasks with dependencies.
