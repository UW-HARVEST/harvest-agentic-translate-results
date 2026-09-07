#!/usr/bin/env bash
# Phase D — build & test every Cargo feature combination, and verify symbol
# parity between the C .so and the Rust .so for each one.
#
# Usage:  ./scripts/feature_matrix.sh [--tests]
#         --tests  also run the full differential test suite per combination
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
C_BUILD="$ROOT/c_src/build"
RUN_TESTS="${1:-}"

cd "$CRATE_DIR"

# --- Build the C library if needed -----------------------------------------
if ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library =="
  mkdir -p "$C_BUILD"
  ( cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(ls "$C_BUILD"/lib*.so | head -1)"

# --- Enumerate the declared features ---------------------------------------
# Everything between the `[features]` header and the next `[section]`.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "" && a[1] !~ /^#/ && a[1] != "default") print a[1] }
  ' Cargo.toml
)

# Build the list of combinations to test.
COMBOS=("default")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("none")
  n=${#FEATURES[@]}
  # Full power set of the non-default features (with --no-default-features).
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$sel")
  done
  # And every single feature ADDED to the default set.
  for f in "${FEATURES[@]}"; do COMBOS+=("default+$f"); done
else
  # No `[features]` section: `--no-default-features` is equivalent to the
  # default build, but exercise it explicitly anyway.
  COMBOS+=("none")
fi

echo "== features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"
echo "== combinations to verify: ${#COMBOS[@]}"

fail=0

flags_for() {
  case "$1" in
    default) echo "" ;;
    none)    echo "--no-default-features" ;;
    default+*) echo "--features ${1#default+}" ;;
    *)       echo "--no-default-features --features $1" ;;
  esac
}

for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  FLAGS=$(flags_for "$combo")
  echo
  echo "=================================================================="
  echo "== combo: $combo   (cargo $FLAGS)"
  echo "=================================================================="

  if ! timeout 600 cargo check $FLAGS >/dev/null 2>&1; then
    echo "  cargo check FAILED"; fail=1; continue
  fi
  echo "  cargo check ok"

  if ! timeout 600 cargo build --release $FLAGS >/dev/null 2>&1; then
    echo "  cargo build --release FAILED"; fail=1; continue
  fi
  R_SO="target/release/libarr_del_lib.so"
  if [ ! -f "$R_SO" ]; then echo "  missing $R_SO"; fail=1; continue; fi
  echo "  cargo build --release ok"

  # --- symbol parity ------------------------------------------------------
  nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort -u > /tmp/fm_c.txt
  nm -D --defined-only "$R_SO" | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort -u > /tmp/fm_r.txt
  missing=$(comm -23 /tmp/fm_c.txt /tmp/fm_r.txt)
  if [ -n "$missing" ]; then
    echo "  SYMBOLS MISSING FROM RUST:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  symbol parity ok ($(wc -l < /tmp/fm_c.txt) symbols, 0 missing)"
  fi

  # Undefined non-libc symbols in the Rust .so.
  undef=$(nm -D -u "$R_SO" | sed 's/^ *[wU] *//' | sed 's/@.*//' | sort -u \
          | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__tls_get_addr|__errno_location)' \
          | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|sprintf|stat64|statx|strcmp|strlen|syscall|write|writev' || true)
  if [ -n "$undef" ]; then
    echo "  UNDEFINED NON-LIBC SYMBOLS:"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  no undefined non-libc symbols"
  fi

  # --- differential tests -------------------------------------------------
  if [ "$RUN_TESTS" = "--tests" ]; then
    for t in phase_b_low phase_b_map phase_c_errors; do
      if timeout 600 cargo test --release $FLAGS --test "$t" -- --test-threads=1 >/tmp/fm_test.log 2>&1; then
        echo "  tests $t: $(grep -oE '[0-9]+ passed' /tmp/fm_test.log | tail -1)"
      else
        echo "  tests $t FAILED"; tail -25 /tmp/fm_test.log | sed 's/^/    /'; fail=1
      fi
    done
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS OK"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
