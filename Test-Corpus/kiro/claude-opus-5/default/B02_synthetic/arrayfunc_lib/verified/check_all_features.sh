#!/usr/bin/env bash
# Enumerate every valid feature combination from Cargo.toml and, for each one:
#   cargo check -> cargo build (cdylib) -> symbol parity vs the C .so -> cargo test
# The crate declares no [features], so the powerset is the single empty combo;
# the loop is written generically so added features are picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)

if [ -z "$C_SO" ]; then
  echo "C .so missing; build it first:"
  echo "  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

# Feature names declared in [features], excluding the `default` meta-feature.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

# Build the powerset of feature names.
COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
  done
  COMBOS+=("$combo")
done
# Also exercise the crate's own default selection.
COMBOS+=("__default__")

echo "features declared: ${n} -> ${#COMBOS[@]} configurations to verify"

overall=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__default__" ]; then
    label="(default features)"; ARGS=()
  elif [ -z "$combo" ]; then
    label="(no features)"; ARGS=(--no-default-features)
  else
    label="$combo"; ARGS=(--no-default-features --features "$combo")
  fi

  echo
  echo "=============================================================="
  echo "CONFIG: $label"
  echo "=============================================================="

  for phase in check build; do
    if ! timeout 600 cargo "$phase" --release "${ARGS[@]}" >/tmp/fc.log 2>&1; then
      echo "  cargo $phase: FAIL"; tail -25 /tmp/fc.log; overall=1; continue 2
    fi
    echo "  cargo $phase: ok"
  done

  RS_SO=target/release/libarrayfunc_lib.so
  # Symbol parity: every symbol the C .so exports must be exported by the Rust
  # .so under the exact same name.
  nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TWDBRVi]$/ {print $3}' | sort -u >/tmp/c_syms.txt
  nm -D --defined-only "$RS_SO" | awk '$2 ~ /^[TWDBRVi]$/ {print $3}' | sort -u >/tmp/r_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
  if [ -n "$missing" ]; then
    echo "  symbol parity: FAIL — missing from Rust .so:"; echo "$missing" | sed 's/^/    /'
    overall=1
  else
    echo "  symbol parity: ok ($(wc -l </tmp/c_syms.txt) C exports all present)"
  fi

  for profile in "--release" ""; do
    pname=$([ -n "$profile" ] && echo release || echo debug)
    if timeout 600 cargo test $profile "${ARGS[@]}" >/tmp/ft.log 2>&1; then
      echo "  cargo test ($pname): ok — $(grep -c '^test .* ok$' /tmp/ft.log) assertions groups passed"
    else
      echo "  cargo test ($pname): FAIL"; grep -E '^test .*(FAILED|ok)$|panicked|assertion' /tmp/ft.log | head -20
      overall=1
    fi
  done
done

echo
if [ "$overall" -eq 0 ]; then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "FAILURES PRESENT"
fi
exit "$overall"
