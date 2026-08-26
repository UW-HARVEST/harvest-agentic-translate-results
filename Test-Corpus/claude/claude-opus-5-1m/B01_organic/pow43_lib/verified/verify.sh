#!/usr/bin/env bash
# Full verification driver for the C -> Rust translation.
#
#   ./verify.sh            # everything (default)
#   ./verify.sh check      # cargo check for every feature combination
#   ./verify.sh test       # cargo test for every feature combination + profile
#   ./verify.sh symbols    # nm -D parity between the C .so and the Rust .so
#   ./verify.sh copt       # differential tests against C built at -O0..-O3/-Os
#
# Every command is wrapped in `timeout` so no single step can hang the run.
set -uo pipefail

cd "$(dirname "$0")"
TMPDIR="${TMPDIR:-/tmp}"
CARGO_FLAGS="--offline"
TIMEOUT=600
rc=0

say() { printf '\n=== %s ===\n' "$*"; }
ok()  { printf '  [ OK ]   %s\n' "$*"; }
bad() { printf '  [FAIL]   %s\n' "$*"; rc=1; }

run() { # run <label> <cmd...>
  local label="$1"; shift
  local log="$TMPDIR/verify-$(echo "$label" | tr -c 'A-Za-z0-9._-' '_').log"
  if timeout "$TIMEOUT" "$@" >"$log" 2>&1; then
    ok "$label"
  else
    bad "$label (log: $log)"
    tail -n 25 "$log" | sed 's/^/          /'
  fi
}

# ---------------------------------------------------------------------------
# Enumerate every valid feature combination, mechanically, from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_.-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

combos=()                                   # each entry: cargo feature flags
combos+=("--no-default-features")           # empty configuration
combos+=("")                                # default configuration
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[b]}")
    done
    combos+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
  combos+=("--all-features")
fi

phase_report() {
  say "feature combinations found (${#combos[@]})"
  for c in "${combos[@]}"; do
    printf '  * cargo <cmd> %s\n' "${c:-<default>}"
  done
  if [ "${#FEATURES[@]}" -eq 0 ]; then
    printf '  (Cargo.toml declares no optional features: one build configuration)\n'
  fi
}

phase_check() {
  say "cargo check, every feature combination"
  for c in "${combos[@]}"; do
    # shellcheck disable=SC2086
    run "check ${c:-<default>}" cargo check $CARGO_FLAGS $c --all-targets
  done
}

phase_test() {
  for profile in "" "--release"; do
    say "cargo test ${profile:-<debug>}, every feature combination"
    for c in "${combos[@]}"; do
      # shellcheck disable=SC2086
      run "test ${profile:-debug} ${c:-<default>}" cargo test $CARGO_FLAGS $profile $c
    done
  done
}

phase_symbols() {
  say "symbol parity (nm -D)"
  local cso="c_src/build/libtranslated_rust.so"
  local rso="target/test-cdylib/debug/libpow43_lib.so"
  [ -f "$cso" ] || run "build C .so" bash -c \
    'mkdir -p c_src/build && cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .'
  [ -f "$rso" ] || run "build Rust cdylib" cargo build $CARGO_FLAGS --lib --target-dir target/test-cdylib
  if [ -f "$cso" ] && [ -f "$rso" ]; then
    local d
    d=$(diff <(nm -D --defined-only "$cso" | awk '{print $NF}' | sort -u) \
             <(nm -D --defined-only "$rso" | awk '{print $NF}' | sort -u))
    if [ -z "$d" ]; then
      ok "exported symbol sets are identical: $(nm -D --defined-only "$cso" | awk '{print $NF}' | tr '\n' ' ')"
    else
      bad "symbol diff (< C only, > Rust only):"; echo "$d" | sed 's/^/          /'
    fi
    if ldd -r "$rso" 2>&1 | grep -q "undefined symbol"; then
      bad "Rust .so has unresolved symbols"
    else
      ok "no unresolved (non-libc) symbols in the Rust .so"
    fi
  fi
}

phase_copt() {
  say "differential tests against the C library at several optimization levels"
  for opt in "" "-O1" "-O2" "-O3" "-Os"; do
    local d="$TMPDIR/verify-cbuild${opt:--O0}"
    rm -rf "$d"
    if timeout "$TIMEOUT" cmake -S c_src -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
         ${opt:+-DCMAKE_C_FLAGS="$opt"} >/dev/null 2>&1 &&
       timeout "$TIMEOUT" cmake --build "$d" >/dev/null 2>&1; then
      C_POW43_SO="$d/libtranslated_rust.so" \
        run "differential vs C ${opt:-default(-O0)}" cargo test $CARGO_FLAGS --test differential
    else
      bad "could not build the C library with ${opt:-default}"
    fi
  done
}

case "${1:-all}" in
  report)  phase_report ;;
  check)   phase_report; phase_check ;;
  test)    phase_report; phase_test ;;
  symbols) phase_symbols ;;
  copt)    phase_copt ;;
  all)     phase_report; phase_check; phase_test; phase_symbols; phase_copt ;;
  *) echo "usage: $0 [all|report|check|test|symbols|copt]" >&2; exit 2 ;;
esac

say "result"
[ "$rc" -eq 0 ] && echo "ALL CHECKS PASSED" || echo "SOME CHECKS FAILED"
exit "$rc"
