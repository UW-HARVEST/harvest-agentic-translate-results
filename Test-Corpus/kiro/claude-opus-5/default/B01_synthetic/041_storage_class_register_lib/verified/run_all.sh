#!/usr/bin/env bash
# Full verification sweep: every feature combination x every build profile.
#
# Phase D requires that the checks hold under EVERY feature configuration, so
# the combinations are extracted from Cargo.toml rather than hard-coded.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
fail=0

echo "=== building the C reference shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 300 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# Enumerate cargo feature combinations declared in Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== Cargo.toml declares no [features]: the only configurations are"
  echo "    the default feature set and --no-default-features ==="
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
  # every non-empty subset of the declared features
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo:--no-default-features --features $combo")
  done
fi

echo "=== cargo check across ${#COMBOS[@]} feature configuration(s) ==="
for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"; flags="${entry#*:}"
  if timeout 600 cargo check $flags >/dev/null 2>&1; then
    echo "  check [$name] OK"
  else
    echo "  check [$name] FAILED"; fail=1
  fi
done

# ---------------------------------------------------------------------------
# Build + differential-test every combination, in both profiles.
# `cargo test` does not build the cdylib, so build it explicitly and hand the
# exact artifact to the tests via DRIVER_RUST_SO.
# ---------------------------------------------------------------------------
for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"; flags="${entry#*:}"
  for profile in debug release; do
    if [ "$profile" = release ]; then pflag="--release"; else pflag=""; fi
    echo "=== [$name / $profile] build cdylib + run differential tests ==="
    if ! timeout 600 cargo build $flags $pflag >/dev/null 2>&1; then
      echo "  build FAILED"; fail=1; continue
    fi
    SO="target/$profile/libdriver.so"
    if [ ! -f "$SO" ]; then echo "  missing $SO"; fail=1; continue; fi
    nm -D --defined-only "$SO" | sed 's/^/  rust sym: /'
    if DRIVER_RUST_SO="$PWD/$SO" timeout 600 cargo test $flags $pflag \
         -- --test-threads=1 2>&1 | tail -5; then
      echo "  tests [$name / $profile] OK"
    else
      echo "  tests [$name / $profile] FAILED"; fail=1
    fi
  done
done

# ---------------------------------------------------------------------------
# Phase D symbol diff, reported directly (also asserted by a test).
# ---------------------------------------------------------------------------
echo "=== Phase D: nm -D symbol diff (C vs Rust) ==="
for profile in debug release; do
  SO="target/$profile/libdriver.so"
  [ -f "$SO" ] || continue
  diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only "$SO"   | awk '{print $NF}' | sort -u) \
    && echo "  [$profile] symbol sets identical" \
    || { echo "  [$profile] SYMBOL DIFF (lines starting '<' are missing from Rust)"; \
         nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/c_syms.$$; \
         nm -D --defined-only "$SO"   | awk '{print $NF}' | sort -u > /tmp/r_syms.$$; \
         if comm -23 /tmp/c_syms.$$ /tmp/r_syms.$$ | grep -q .; then fail=1; fi; \
         rm -f /tmp/c_syms.$$ /tmp/r_syms.$$; }
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
