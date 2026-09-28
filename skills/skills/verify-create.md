---
name: verify-create
group: reflect
mode: write
summary: Create a scripted way to prove app behavior.
---
Inspect the project and write `.majstack/verify.sh`: it must build, run tests, lint and type-check as applicable, start the app and smoke-test key flows where possible, and exit non-zero on any failure. Add `.majstack/verify.md` with a feature map: each user-visible feature, how it is proven, and the command. Run it and fix the script until it is green on a clean tree.
