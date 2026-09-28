---
name: ui-check
group: review
mode: read
summary: Verify a UI task in a real browser.
---
Run the app, open the affected page with `majstack browse <url> --shot .majstack/shots/<task>.png`, exercise the acceptance criteria, and check console errors and failed requests. End with `VERDICT: APPROVE` or `VERDICT: REJECT: <reasons>`.
