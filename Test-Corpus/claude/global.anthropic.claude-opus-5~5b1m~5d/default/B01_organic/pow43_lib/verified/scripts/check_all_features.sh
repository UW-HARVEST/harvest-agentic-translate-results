#!/bin/bash
# Phase D — run the whole verification under every build configuration.
#
# Feature axis: the feature powerset is read out of Cargo.toml rather than
# hard-coded, so this keeps working if features are ever added. With no
# [features] section the powerset is empty and only the canonical flag
# combinations remain.
#
# Profile axis: the cdylib is also built with the dev profile (which enables
# `overflow-checks`) and the whole suite is re-run against it, because a
# translation that used plain `*`/`+` instead of `wrapping_*` would panic there
# while passing in release.
set -u
cd "$(dirname "$0")/.." || exit 1

features=$(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        n = line.split('=', 1)[0].strip().strip('"')
        if n and n != 'default':
            names.append(n)
print(' '.join(names))
PY
)

echo "declared features: ${features:-<none>}"
combos=()
combos+=("")                                  # default
combos+=("--no-default-features")
combos+=("--all-features")
combos+=("--no-default-features --all-features")
if [ -n "$features" ]; then
  # Full powerset of explicitly declared features, with and without defaults.
  n=0; for f in $features; do n=$((n+1)); done
  total=$((1 << n))
  for ((mask=1; mask<total; mask++)); do
    sel=""; i=0
    for f in $features; do
      if (( (mask >> i) & 1 )); then sel="$sel,$f"; fi
      i=$((i+1))
    done
    sel="${sel#,}"
    combos+=("--features $sel")
    combos+=("--no-default-features --features $sel")
  done
fi

fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<default>}"
  printf '=== cargo check %s\n' "$label"
  if ! cargo check --all-targets $combo >/dev/null 2>&1; then
    echo "    CHECK FAILED"; fail=1; continue
  fi
  printf '=== cargo test  %s\n' "$label"
  out=$(POW43_CARGO_FEATURE_ARGS="$combo" timeout 600 cargo test --release $combo 2>&1)
  if printf '%s' "$out" | grep -qE 'FAILED|SIGSEGV|SIGBUS|test result: FAILED'; then
    echo "    TEST FAILED"; printf '%s' "$out" | grep -E 'FAILED|panicked' | head -5 | sed 's/^/      /'; fail=1
  else
    printf '%s' "$out" | grep -E '^test result' | sed 's/^/      /'
  fi
done

echo
echo "=== profile axis: run suite against a dev-profile (overflow-checks) cdylib"
cargo build --target-dir target/dbg >/dev/null 2>&1 || { echo "    dev build FAILED"; fail=1; }
DBG=$(find target/dbg/debug -maxdepth 1 -name 'lib*.so' -type f | sort | head -1)
if [ -n "$DBG" ]; then
  out=$(POW43_RUST_SO="$PWD/$DBG" timeout 600 cargo test --release 2>&1)
  if printf '%s' "$out" | grep -qE 'FAILED|SIGSEGV|SIGBUS'; then
    echo "    TEST FAILED against dev-profile .so"; printf '%s' "$out" | grep -E 'FAILED|panicked' | head -5 | sed 's/^/      /'; fail=1
  else
    printf '%s' "$out" | grep -E '^test result' | sed 's/^/      /'
  fi
else
  echo "    no dev-profile .so produced"; fail=1
fi

echo
if [ "$fail" -eq 0 ]; then echo "RESULT: PASS — all configurations verified"; else echo "RESULT: FAIL"; fi
exit "$fail"
