#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
#
# Usage:  ./verify.sh [symbols|combo <flags...>|all]
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_BUILD="$ROOT/c_src/build"

die() { echo "FAIL: $*" >&2; exit 1; }

build_c() {
  [ -d "$C_BUILD" ] || mkdir -p "$C_BUILD"
  ( cd "$C_BUILD" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || die "C build failed"
}

c_so() { find "$C_BUILD" -maxdepth 1 -name '*.so' | sort | tail -n1; }
rs_so() { echo "$CRATE_DIR/target/release/libhm_geti_lib.so"; }

syms() { nm -D --defined-only "$1" | awk '{print $3}' | sort -u; }

check_symbols() {
  local label="$1"
  local c r missing extra
  c="$(syms "$(c_so)")"
  r="$(syms "$(rs_so)")"
  missing="$(comm -23 <(echo "$c") <(echo "$r"))"
  extra="$(comm -13 <(echo "$c") <(echo "$r"))"
  echo "[$label] C exports $(echo "$c" | wc -l), Rust exports $(echo "$r" | wc -l)"
  if [ -n "$missing" ]; then
    echo "[$label] MISSING FROM RUST:"; echo "$missing"
    die "$label: symbol diff is not empty"
  fi
  echo "[$label] symbol diff: EMPTY (all C symbols exported by Rust)"
  [ -n "$extra" ] && { echo "[$label] Rust-only extra symbols (allowed):"; echo "$extra"; }
  # no undefined non-libc symbols
  local bad
  bad="$(nm -D --undefined-only "$(rs_so)" | awk '{print $2}' | sed 's/@.*//' \
        | grep -vE '^(_ITM_|__gmon_start__|_Unwind_|__cxa_|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strcmp|strlen|syscall|write|writev)' || true)"
  [ -n "$bad" ] && { echo "[$label] unexpected undefined symbols:"; echo "$bad"; die "$label: undefined non-libc symbols"; }
  echo "[$label] undefined non-libc symbols: NONE"
}

feature_combos() {
  # Enumerate every feature combination declared in Cargo.toml. With no
  # [features] section the only combination is the default build, but the loop is
  # written generically so adding features cannot silently skip coverage.
  local feats
  feats="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,""); gsub(/ /,""); if ($0!="") print}' "$CRATE_DIR/Cargo.toml")"
  echo "DEFAULT"
  echo "NO_DEFAULT"
  if [ -n "$feats" ]; then
    echo "ALL"
    local n=0 combo
    # every non-empty subset, on top of --no-default-features
    local arr=($feats)
    local total=$((1 << ${#arr[@]}))
    for ((mask=1; mask<total; mask++)); do
      combo=""
      for ((i=0; i<${#arr[@]}; i++)); do
        (( mask & (1<<i) )) && combo="$combo,${arr[$i]}"
      done
      echo "SUBSET:${combo#,}"
      n=$((n+1))
    done
    echo "($n explicit feature subsets)" >&2
  fi
}

flags_for() {
  case "$1" in
    DEFAULT)    echo "" ;;
    NO_DEFAULT) echo "--no-default-features" ;;
    ALL)        echo "--all-features" ;;
    SUBSET:*)   echo "--no-default-features --features ${1#SUBSET:}" ;;
  esac
}

run_combo() {
  local combo="$1"; shift
  local flags; flags="$(flags_for "$combo")"
  echo "=============================================================="
  echo "COMBO: $combo   (cargo flags: '${flags:-<none>}')"
  echo "=============================================================="
  # shellcheck disable=SC2086
  cargo check $flags >/dev/null 2>&1 || die "$combo: cargo check failed"
  echo "[$combo] cargo check: ok"
  # shellcheck disable=SC2086
  cargo build --release $flags >/dev/null 2>&1 || die "$combo: release build failed"
  check_symbols "$combo"
  if [ "${1:-tests}" = "tests" ]; then
    local log rc
    for t in phase_b_low phase_b_hashmap phase_b_driver phase_b_stress phase_c_errors; do
      log="$(mktemp)"
      # shellcheck disable=SC2086
      timeout 580 cargo test $flags --test "$t" -- --test-threads=1 >"$log" 2>&1
      rc=$?
      grep -E '^test result' "$log" | sed "s/^/[$combo:$t] /"
      if [ $rc -ne 0 ]; then
        echo "--- tail of $log ---"; tail -n 40 "$log"
        die "$combo: $t FAILED (rc=$rc)"
      fi
      rm -f "$log"
    done
  fi
}

cd "$CRATE_DIR"
build_c

case "${1:-all}" in
  symbols)
    cargo build --release >/dev/null 2>&1 || die "release build failed"
    check_symbols DEFAULT
    ;;
  combos)
    feature_combos
    ;;
  combo)
    shift
    run_combo "$1" "${2:-tests}"
    ;;
  all)
    while read -r combo; do
      run_combo "$combo" tests
    done < <(feature_combos)
    echo "ALL COMBINATIONS PASSED"
    ;;
  *) die "unknown mode: $1" ;;
esac
