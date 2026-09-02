#!/usr/bin/env bash
# Phase D: symbol parity + full suite under EVERY feature combination.
#
# Enumerates the features declared in Cargo.toml, builds the cdylib and runs the
# whole differential suite for each combination, and diffs `nm -D` against the C
# .so every time.
set -u
cd "$(dirname "$0")"

C_SO=../c_src/build/libdriver.so
RS_SO=target/release/libdriver.so

if [[ ! -f $C_SO ]]; then
  echo "FATAL: $C_SO missing; build the C library first" >&2
  exit 1
fi

# --- enumerate feature combinations -----------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        n = line.split('=')[0].strip().strip('"')
        if n != 'default':
            names.append(n)
print('\n'.join(names))
PY
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 || -z ${FEATURES[0]:-} ]]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS+=("DEFAULT")
  COMBOS+=("NONE")
else
  COMBOS+=("DEFAULT")
  COMBOS+=("NONE")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

# --- run each combination ---------------------------------------------------
status=0
c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) flags=() ; label="default" ;;
    NONE)    flags=(--no-default-features) ; label="--no-default-features" ;;
    *)       flags=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac

  printf '\n=== %s ===\n' "$label"

  if ! timeout 600 cargo build --release "${flags[@]}" >/tmp/fc_build.log 2>&1; then
    echo "  BUILD FAILED"; tail -5 /tmp/fc_build.log; status=1; continue
  fi

  rs_syms=$(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rs_syms"))
  if [[ -n $missing ]]; then
    echo "  SYMBOL PARITY FAILED — missing from Rust .so:"; echo "$missing" | sed 's/^/    /'
    status=1
  else
    echo "  symbol parity: OK ($(echo "$c_syms" | wc -l) C symbol(s), 0 missing)"
  fi

  undef=$(nm -D -u "$RS_SO" | awk '$1=="U"{print $NF}' | sed 's/@.*//' | sort -u)
  echo "  undefined in Rust .so (all must be libc/libgcc): $(echo "$undef" | wc -l) symbols"

  if timeout 600 cargo test --release "${flags[@]}" >/tmp/fc_test.log 2>&1; then
    ok_suites=$(grep -c 'test result: ok' /tmp/fc_test.log)
    n_tests=$(grep -oP '\d+(?= passed)' /tmp/fc_test.log | awk '{s+=$1} END {print s+0}')
    echo "  tests: $ok_suites suites ok, $n_tests tests passed"
  else
    echo "  TESTS FAILED"; grep -E 'FAILED|panicked|test result' /tmp/fc_test.log | head -20
    status=1
  fi
done

printf '\n===============================\n'
if [[ $status -eq 0 ]]; then echo "ALL COMBINATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit $status
