#!/usr/bin/env bash
# Full differential verification: build the C .so, build the Rust cdylib, check
# symbol parity, and run Phase B + Phase C tests under every feature combination.
#
# NOTE: `cargo test` does NOT relink the cdylib (the integration tests do not
# link against it), so `cargo build` MUST precede every `cargo test` run. The
# harness also asserts this (see assert_not_stale in tests/harness/mod.rs).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
CARGO_FLAGS="--offline"
PROFILE="${PROFILE:-release}"
[[ "$PROFILE" == "release" ]] && PROF_FLAG="--release" || PROF_FLAG=""

echo "=== 1. build C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

echo
echo "=== 2. enumerate feature combinations from Cargo.toml ==="
# Every feature declared in [features], if any.
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{gsub(/ /,"");split($0,a,"=");print a[1]}' \
        "$HERE/Cargo.toml" | grep -v '^default$' || true)
if [[ -z "$FEATS" ]]; then
  echo "no [features] declared -> the only configurations are default / --no-default-features"
  COMBOS=("" "--no-default-features")
else
  # Full power set of declared features, each also with --no-default-features.
  mapfile -t FA <<< "$FEATS"
  COMBOS=("" "--no-default-features")
  n=${#FA[@]}
  for ((m=1; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do (( m & (1<<i) )) && sel="${sel:+$sel,}${FA[i]}"; done
    COMBOS+=("--features $sel" "--no-default-features --features $sel")
  done
fi
printf '  combo: [%s]\n' "${COMBOS[@]}"

FAIL=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "############################################################"
  echo "### CONFIG: cargo ... $PROF_FLAG $combo"
  echo "############################################################"

  echo "--- build Rust cdylib ---"
  # shellcheck disable=SC2086
  cargo build $CARGO_FLAGS $PROF_FLAG $combo
  R_SO="$HERE/target/$PROFILE/libdriver.so"

  echo "--- symbol parity (nm -D) ---"
  c_syms=$(nm -D "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u)
  r_syms=$(nm -D "$R_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms") || true)
  if [[ -n "$missing" ]]; then
    echo "MISSING from Rust .so:"; echo "$missing"; FAIL=1
  else
    echo "OK: every C-exported symbol is exported by the Rust .so"
    echo "$c_syms" | sed 's/^/    /'
  fi
  echo "--- undefined non-libc symbols in Rust .so ---"
  nm -D "$R_SO" | awk '$1=="U"{print $2}' | sed 's/@.*//' | sort -u | sed 's/^/    U /'

  echo "--- Phase B + C differential tests ---"
  # shellcheck disable=SC2086
  cargo test $CARGO_FLAGS $PROF_FLAG $combo -- --test-threads=1 || FAIL=1
done

echo
if [[ $FAIL -eq 0 ]]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES DETECTED"; exit 1; fi
