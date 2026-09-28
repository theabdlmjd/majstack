---
name: investigate
group: build
mode: read
summary: Root-cause debugging: no fixes without investigation.
---
Iron rule: no fix before investigation. Reproduce the symptom, trace data flow, form one hypothesis at a time, test each with evidence, and stop after three failed hypotheses to report. Freeze edits to the module under investigation (`majstack guard freeze <dir>`) if it helps. Deliver a report: symptom, root cause with proof, proposed fix, and a regression test to add.
