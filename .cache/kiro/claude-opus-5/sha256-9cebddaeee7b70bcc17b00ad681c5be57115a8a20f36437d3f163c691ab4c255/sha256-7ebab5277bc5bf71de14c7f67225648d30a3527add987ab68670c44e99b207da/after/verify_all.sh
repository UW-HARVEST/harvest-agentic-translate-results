#!/usr/bin/env bash
# Enumerates every feature combination declared in translation/Cargo.toml and
# runs `cargo check` then `cargo test` for each. Run from the repo root.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT/translation"

# --- extract feature names from the [features] section --------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

echo "features declared: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# --- build the combination list -------------------------------------------
COMBOS=("")   # always test --no-default-features with nothing enabled
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]}"

# --- ensure the C library exists ------------------------------------------
if ! ls ../c_src/build/*.so >/dev/null 2>&1; then
  echo "building C library..."
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

FAIL=0

run() {  # run <label> <logfile> <cmd...>
  local label="$1" log="$2"; shift 2
  if timeout 600 "$@" >"$log" 2>&1; then
    echo "  PASS  $label"
  else
    echo "  FAIL  $label  (see $log)"
    tail -n 25 "$log" | sed 's/^/        /'
    FAIL=1
  fi
}

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo "=== $label ==="
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  slug="${combo//,/_}"; slug="${slug:-none}"

  run "cargo check          [$label]" "/tmp/ima-check-$slug.log" \
      cargo check "${args[@]}" --all-targets
  run "cargo build --release[$label]" "/tmp/ima-build-$slug.log" \
      cargo build --release "${args[@]}"
  # The test harness loads the cdylib from target/<profile>/, so build it for
  # this feature set before the tests run.
  IMA_TEST_FEATURES="$combo" \
    run "cargo test --release[$label]" "/tmp/ima-test-$slug.log" \
        cargo test --release "${args[@]}"
  IMA_TEST_FEATURES="$combo" \
    run "cargo test  (debug) [$label]" "/tmp/ima-testdbg-$slug.log" \
        cargo test "${args[@]}"

  # --- symbol parity ------------------------------------------------------
  c_so="$(ls ../c_src/build/*.so | head -n1)"
  r_so=target/release/libima_parse_lib.so
  c_syms="$(nm -D --defined-only "$c_so" | awk '$2 ~ /^[TtDBRW]$/ {print $3}' | sort -u)"
  r_syms="$(nm -D --defined-only "$r_so" | awk '$2 ~ /^[TtDBRW]$/ {print $3}' | sort -u)"
  missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
  if [[ -n "$missing" ]]; then
    echo "  FAIL  symbol parity [$label]: Rust .so missing: $(echo "$missing" | tr '\n' ' ')"
    FAIL=1
  else
    echo "  PASS  symbol parity [$label]"
  fi
done

if (( FAIL )); then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all combinations verified"
