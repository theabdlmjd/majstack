---
name: blast-radius
group: plan
mode: read
summary: Find what else a change could break, and prove why it is safe.
---
List every caller, consumer, config, script, doc and test the change touches, including indirect ones. For each, state the risk. Then identify the one fact the change is safe because of, and prove it by running code, not by assertion. Rate overall risk low, medium or high and name the tests that must pass.
