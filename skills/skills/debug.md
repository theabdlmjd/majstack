---
name: debug
group: build
mode: write
summary: Fix a failed verification by finding the root cause first.
---
Verification or review failed; the failure output is attached. Reproduce it, ask why repeatedly until you reach the cause, and fix that, not the symptom. Do not silence errors with guards. Add a regression test. After 3 failed fix attempts on the same task, stop and report what you learned instead of guessing again.
