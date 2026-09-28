#!/usr/bin/env bash
prompt="$(cat)"
if printf '%s' "$prompt" | grep -q '# PLANNING'; then
  echo '<tasks>[{"id":"1","title":"create demo files","depends":[]}]</tasks>'
  echo 'MAJSTACK_DONE: planned'
  exit 0
fi
if printf '%s' "$prompt" | grep -q 'review the task diff'; then
  echo 'VERDICT: APPROVE'
  echo 'MAJSTACK_DONE: reviewed'
  exit 0
fi
printf 'alpha\n' > a.txt
printf 'beta\n' > b.txt
echo 'MAJSTACK_LEARN: PATTERN: demo artifacts are plain text files'
echo 'MAJSTACK_DONE: created demo files'
