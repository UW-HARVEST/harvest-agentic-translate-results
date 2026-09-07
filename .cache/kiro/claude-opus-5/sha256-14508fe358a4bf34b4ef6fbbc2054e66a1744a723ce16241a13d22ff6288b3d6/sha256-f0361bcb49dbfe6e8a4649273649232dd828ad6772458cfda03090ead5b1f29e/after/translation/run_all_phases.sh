#!/usr/bin/env bash
# Phase D driver: enumerate every Cargo feature combination, build the cdylib
# for each, and run all three phases against it. Also runs each phase against
# the debug-profile .so, whose codegen settings (overflow checks on, unwinding
# panics) differ from the release profile's `panic = "abort"`.
set -uo pipefail
cd "$(dirname "$0")"

fail=0
run() { # run <label> <rust .so> <extra cargo args...>
  local label="$1"; shift
  local so="$1"; shift
  echo "=================================================================="
  echo "## $label"
  echo "##   RUST_SO=$so  cargo test $*"
  echo "=================================================================="
  if RUST_SO="$so" timeout 600 cargo test "$@" -- --test-threads=1 2>&1 \
      | grep -E "^(test result|error|warning: unused|test .* FAILED)"; then :; fi
  # shellcheck disable=SC2181
  if ! RUST_SO="$so" timeout 600 cargo test "$@" -- --test-threads=1 >/dev/null 2>&1; then
    echo "!! FAILED: $label ($*)"
    fail=1
  fi
}

# ---- enumerate feature combinations from Cargo.toml -----------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,""); split($0,a,"="); if (a[1] != "default") print a[1]}' Cargo.toml
)
echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]-none})"

# Combination list: always the default build, plus no-default-features and
# all-features, plus the powerset of declared features.
COMBOS=("")
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
fi

for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo >/dev/null 2>&1 || { echo "!! release build failed: $combo"; fail=1; continue; }
  # shellcheck disable=SC2086
  timeout 600 cargo build $combo >/dev/null 2>&1 || { echo "!! debug build failed: $combo"; fail=1; continue; }

  # Symbol parity must hold for this combination's artifact.
  csyms=$(nm -D --defined-only --format=posix ../c_src/build/*.so | awk '$2=="T"{print $1}' | sort)
  rsyms=$(nm -D --defined-only --format=posix target/release/libcharinbuf_lib.so | awk '$2=="T"{print $1}' | sort)
  miss=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  if [[ -n "$miss" ]]; then
    echo "!! MISSING SYMBOLS for combo '${combo:-<default>}':"; echo "$miss"; fail=1
  else
    echo "-- symbol diff empty for combo '${combo:-<default>}' ($(echo "$csyms" | wc -l) symbols)"
  fi

  # shellcheck disable=SC2086
  run "combo '${combo:-<default>}' / release .so" "$PWD/target/release/libcharinbuf_lib.so" $combo
  # shellcheck disable=SC2086
  run "combo '${combo:-<default>}' / debug .so"   "$PWD/target/debug/libcharinbuf_lib.so"   $combo
done

echo "=================================================================="
if (( fail )); then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: all combinations passed"
