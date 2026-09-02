#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, then run the differential suite under
# EVERY cargo feature combination, in both the debug and release profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a feature automatically widens the matrix.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$here"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
note "building C shared library"
( mkdir -p "$root/c_src/build" \
  && cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so="$(ls "$root"/c_src/build/lib*.so)"
echo "C .so: $c_so"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
features=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[ \t]*=/ {
      split($0, a, "="); gsub(/[ \t]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

combos=("")                       # default feature set
combos+=("--no-default-features")
if [ -n "$features" ]; then
  combos+=("--all-features")
  # every individual feature, on its own, with defaults off
  while read -r f; do
    [ -n "$f" ] && combos+=("--no-default-features --features $f")
  done <<< "$features"
  # and the full power set of the declared features
  list=($features)
  n=${#list[@]}
  if [ "$n" -gt 0 ] && [ "$n" -le 10 ]; then
    for ((mask=1; mask<(1<<n); mask++)); do
      sel=""
      for ((i=0; i<n; i++)); do
        if (( mask & (1<<i) )); then sel="$sel,${list[$i]}"; fi
      done
      combos+=("--no-default-features --features ${sel#,}")
    done
  fi
else
  echo "Cargo.toml declares no [features]; the matrix is {default, --no-default-features}."
  combos+=("--all-features")
fi

# de-duplicate
mapfile -t combos < <(printf '%s\n' "${combos[@]}" | awk '!seen[$0]++')

note "feature matrix (${#combos[@]} combinations)"
for c in "${combos[@]}"; do echo "  cargo test ${c:-<default>}"; done

# ---------------------------------------------------------------------------
# 3. cargo check every combination first (cheap failure detection)
# ---------------------------------------------------------------------------
for c in "${combos[@]}"; do
  note "cargo check $c"
  if ! timeout 300 cargo check --all-targets $c 2>&1 | tail -3; then
    echo "CHECK FAILED for combo '$c'"; fail=1
  fi
done

# ---------------------------------------------------------------------------
# 4. Run the full differential suite for every combination x profile
# ---------------------------------------------------------------------------
for profile in release debug; do
  pflag=""; [ "$profile" = release ] && pflag="--release"
  for c in "${combos[@]}"; do
    note "cargo test $pflag $c  (profile=$profile)"
    log="/tmp/difftest-$profile-$(echo "${c:-default}" | tr -c 'A-Za-z0-9' '_').log"
    # The cdylib is what the tests dlopen, and `cargo test` alone does not
    # emit it for a cdylib-only crate, so build it explicitly first.
    if ! timeout 300 cargo build $pflag $c >"$log" 2>&1; then
      echo "  BUILD FAILED (combo='$c' profile=$profile) — see $log"; fail=1; continue
    fi
    if timeout 600 cargo test $pflag $c -- --test-threads=1 >"$log" 2>&1; then
      grep -E '^test result:' "$log" | sed 's/^/  /'
    else
      echo "  TEST FAILED (combo='$c' profile=$profile) — see $log"
      grep -E '^(test |error|assertion|thread)' "$log" | grep -v ' ok$' | head -40 | sed 's/^/  /'
      fail=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 5. Symbol parity, checked independently of the test binary
# ---------------------------------------------------------------------------
for profile in release debug; do
  rust_so="target/$profile/libcomplexmode_lib.so"
  [ -f "$rust_so" ] || { echo "missing $rust_so"; fail=1; continue; }
  note "symbol diff: C vs $rust_so"
  missing=$(comm -23 \
    <(nm -D --defined-only --format=posix "$c_so"   | awk '{print $1}' | sort -u) \
    <(nm -D --defined-only --format=posix "$rust_so" | awk '{print $1}' | sort -u))
  if [ -n "$missing" ]; then
    echo "MISSING from Rust .so:"; echo "$missing" | sed 's/^/  /'; fail=1
  else
    echo "  no missing symbols"
  fi
done

note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
