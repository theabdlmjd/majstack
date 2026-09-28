---
name: tdd
group: build
mode: write
summary: Write the failing test first, then the fix.
---
Write a test that fails for the right reason, run it and show the failure, then write the smallest change that makes it pass, then run the whole relevant suite. Never weaken an existing test. If no cheap local test path exists, say so and use the strongest available proof instead.
