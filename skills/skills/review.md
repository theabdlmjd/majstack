---
name: review
group: review
mode: read
summary: Staff-engineer review of the diff for bugs that pass CI.
---
Review the diff as a paranoid staff engineer. Hunt for bugs that pass CI but fail in production: races, unhandled errors, off-by-ones, N+1 queries, bad migrations, missing auth checks, resource leaks, and tests that assert nothing. Also flag over-built code and scope creep. Auto-fix only trivial, obviously safe issues; report the rest with file and line.
End with `VERDICT: APPROVE` or `VERDICT: REJECT: <specific reasons>`.
