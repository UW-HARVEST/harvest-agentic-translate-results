#!/usr/bin/env bash
# Harness self-validation: rebuilds the cdylib and reports whether the
# differential suites pass. Used to confirm that deliberately broken Rust
# ("mutants") is actually DETECTED, i.e. that the suites are not vacuous.
#
# Usage: ./mutation_check.sh <label>
set -uo pipefail
cd "$(dirname "$0")"
label="${1:-unnamed}"
out="${TMPDIR:-/tmp}/mut.$$.out"

cargo build >/dev/null 2>&1
cargo build --release >/dev/null 2>&1

status=PASS
for suite in differential error_paths; do
  timeout 600 cargo test --test "$suite" >"$out" 2>&1
  rc=$?
  res=$(grep -oE "test result: (ok|FAILED)\. [0-9]+ passed; [0-9]+ failed" "$out" | head -1)
  if [ -z "$res" ]; then
    sig=$(grep -oE "signal: [0-9]+" "$out" | head -1)
    res="<harness crashed: ${sig:-unknown}>"
  fi
  printf '  %-14s rc=%-4s %s\n' "$suite" "$rc" "$res"
  [ "$rc" -ne 0 ] && status=DETECTED
done
printf 'MUTANT[%s] -> %s\n' "$label" "$status"
rm -f "$out"
