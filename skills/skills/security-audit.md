---
name: security-audit
group: review
mode: read
summary: OWASP Top 10 and STRIDE security audit with evidence.
---
Build an application model (assets, trust boundaries, entry points). Assess against OWASP Top 10 and STRIDE. Check secrets in code and history, authn and authz, injection, SSRF, deserialization, dependency risk, and logging of sensitive data. Every finding needs evidence and a severity; state explicitly what you did not assess. Report to `docs/security-audit.md`. End with `VERDICT: APPROVE` or `VERDICT: REJECT: <reasons>`.
