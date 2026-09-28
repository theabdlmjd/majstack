# Decision providers (typed decisions)

The engine has two kinds of model use:

1. **Coding agents** — anything that can edit files. Configured under `[agents.*]` in
   `.majstack/config.toml`. Any CLI works (`claude`, `codex`, `opencode`, or your own
   `cmd = [...]`). This is the executor.
2. **Decision models** — fast, typed answers used at judgment points: routing a goal to a work
   type, classifying a failure, gating a review, checking acceptance criteria. Configured under
   `[typed]`.

The decision layer talks to one trait (`SystemOne`: state + questions in, typed answers with
confidence out) and ships two backends, so it is not tied to any single vendor or model.

## Backend: TypeSafe (Jev)

Jev is TypeSafe AI's System One model. It returns typed `choice`, `score`, and `noul` (yes/no)
answers with calibrated confidence, typically in about 100 ms.

```toml
[typed]
enabled = true
provider = "typesafe"
base_url = "https://api.typesafe.ai/v1/systemone"
model = "jev-latest"
api_key_env = "TYPESAFE_API_KEY"
confidence_threshold = 0.6
```

Set `TYPESAFE_API_KEY`, then `majstack ask` or any run that chooses a work type uses Jev.

## Backend: any OpenAI-compatible model (open source friendly)

Point `[typed]` at any OpenAI-compatible chat endpoint: Ollama, vLLM, llama.cpp server,
LM Studio, OpenRouter, or a hosted API. The client requests `response_format: json_object` and
parses the typed answer JSON. This works with small open-source models (Qwen, Llama, Gemma,
DeepSeek, SmolLM) and with constrained-decoding stacks (Outlines, XGrammar, Guidance, SGLang)
if your server exposes them.

```toml
[typed]
enabled = true
provider = "openai"
base_url = "http://localhost:11434/v1"   # Ollama default; vLLM/llama.cpp/LM Studio similar
model = "qwen2.5:7b-instruct"
api_key_env = "OPENAI_API_KEY"           # optional; leave unset for local servers
confidence_threshold = 0.6
```

`majstack ask` uses whichever backend `[typed]` selects, with `--model` to override for one call.

## Asking a typed question directly

```bash
majstack ask "the export duplicates rows after a retry" --choice "feature,bug_fix,investigation,performance" --instructions "Classify this request"
majstack ask "<text>" --noul "Does this convey urgency?"
majstack ask "<text>" --score "calm,frustrated,angry" --instructions "How frustrated is the user?"
majstack ask "<text>" --noul "Is this ambiguous?" --json
```

## Where decisions are used

- **Routing:** `classify_work_type` picks the work type; below the confidence threshold the
  engine falls back to the deterministic keyword router, so a weak decision never derails a run.
- **Failure handling:** failure classification selects the recovery plan (retry, change model,
  reduce scope, replan, escalate).
- **Verification/review:** `verify_acceptance` and `noul` gates check whether a change satisfies
  acceptance criteria, with confidence-gated escalation (`Gated::Act` vs `Gated::Escalate`).

## Why this matters for launching

The decision layer is optional and pluggable. With no `[typed]` backend enabled, the engine
runs purely on the deterministic router and the keyword classifier. Enable a backend when you
want model-judged routing or review, and swap backends without changing the engine.
