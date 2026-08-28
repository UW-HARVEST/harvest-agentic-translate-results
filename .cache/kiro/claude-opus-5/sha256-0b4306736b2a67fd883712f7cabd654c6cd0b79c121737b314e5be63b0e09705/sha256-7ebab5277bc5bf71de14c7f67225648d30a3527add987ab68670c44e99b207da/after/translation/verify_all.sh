#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml and run
# `cargo check` + `cargo test` for each, in both dev and release profiles.
set -uo pipefail

cd "$(dirname "$0")"

# --- enumerate features -----------------------------------------------------
# Parse the [features] table (excluding "default") from Cargo.toml.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/ {inf=0}
    inf && /=/ {
      split($0, a, "=");
      gsub(/[ \t]/, "", a[1]);
      if (a[1] != "" && a[1] != "default" && a[1] !~ /^#/) print a[1];
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBOS=("")   # only the empty combination exists
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "features declared: ${n} (${FEATURES[*]:-none})"
echo "combinations to verify: ${#COMBOS[@]}"

fail=0
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="features='${combo:-<none>}' profile='${profile:-dev}'"
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    [ -n "$profile" ] && args+=("$profile")

    echo "=== cargo check ${label}"
    if ! timeout 600 cargo check "${args[@]}" >/tmp/check.log 2>&1; then
      echo "CHECK FAILED: ${label}"; tail -30 /tmp/check.log; fail=1; continue
    fi

    echo "=== cargo test  ${label}"
    if ! timeout 600 cargo test "${args[@]}" >/tmp/test.log 2>&1; then
      echo "TEST FAILED: ${label}"; tail -40 /tmp/test.log; fail=1; continue
    fi
    grep -E "^test result:" /tmp/test.log | sed 's/^/    /'

    # --- symbol parity ------------------------------------------------------
    outdir=target/release
    [ -z "$profile" ] && outdir=target/debug
    c_syms=$(nm -D --defined-only ../c_src/build/libString_Slice.so | awk '{print $3}' | sort -u)
    r_syms=$(nm -D --defined-only "$outdir/libString_Slice.so" | awk '{print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      echo "MISSING EXPORTS in Rust .so (${label}):"; echo "$missing"; fail=1
    else
      echo "    symbols: all $(echo "$c_syms" | wc -l) C export(s) present in Rust .so"
    fi
  done
done

if [ "$fail" -ne 0 ]; then
  echo "RESULT: FAILURES"
  exit 1
fi
echo "RESULT: all combinations verified"
