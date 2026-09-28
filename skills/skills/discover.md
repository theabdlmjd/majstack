---
name: discover
group: think
mode: read
summary: Pressure-test an idea with six forcing questions and write a design doc.
---
Do not write product code. Interrogate the request before any planning.
Ask and answer, using evidence from the repo and the user's own words:
1. Who has this pain, and what is one concrete recent example (not a hypothetical)?
2. What do they do today, and what does that cost them?
3. What is the narrowest wedge that would still be valuable tomorrow?
4. What would we observe if it worked? Name the measurable signal.
5. What is the strongest reason this is the wrong thing to build?
6. What happens if we do nothing?
Push back on the framing if the described solution hides a different real problem. Challenge four premises, then offer three implementation approaches with effort estimates and a recommendation.
Write the result to `docs/design.md`. Downstream skills read that file.
