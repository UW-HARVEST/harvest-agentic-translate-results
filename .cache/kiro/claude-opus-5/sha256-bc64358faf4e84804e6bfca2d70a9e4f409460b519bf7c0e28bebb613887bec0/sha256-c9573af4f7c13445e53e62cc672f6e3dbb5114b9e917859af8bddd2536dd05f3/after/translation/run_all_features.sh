#!/usr/bin/env bash
# Phase D driver: symbol parity + full test matrix over every feature combination.
#
# Usage:  ./run_all_features.sh        (run from translation/)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
C_SO="$ROOT/c_src/build/libdriver.so"
cd "$CRATE_DIR"

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------- build the C .so
note "Building the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
[[ -f "$C_SO" ]] || { echo "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------- feature combos
# Enumerate every feature from Cargo.toml; with none declared the matrix is the
# default build plus --no-default-features (which are identical here).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/       { inf=1; next }
    /^\[/                 { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0,a,"="); gsub(/[[:space:]]/,"",a[1]); if (a[1] != "default") print a[1] }
  ' Cargo.toml
)

COMBOS=("" "--no-default-features")
# TB_TEST_FEATURES lets the in-test nested build match the combo under test.
if (( ${#FEATURES[@]} > 0 )); then
  echo "Declared features: ${FEATURES[*]}"
  n=${#FEATURES[@]}
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do (( mask>>i & 1 )) && sel+=("${FEATURES[i]}"); done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
else
  echo "Cargo.toml declares no [features]; matrix = default + --no-default-features"
fi

# ---------------------------------------------------------------- matrix
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${profile:-<dev>} ${combo:-<default features>}"
    note "$label"

    # Tell the in-test nested build which features to use.
    case "$combo" in
      "")                      export TB_TEST_FEATURES="" ;;
      "--no-default-features") export TB_TEST_FEATURES="__none__" ;;
      *--features*)            export TB_TEST_FEATURES="${combo##*--features }" ;;
      *)                       export TB_TEST_FEATURES="" ;;
    esac

    # shellcheck disable=SC2086
    timeout 600 cargo build $profile $combo >/dev/null 2>&1 \
      || { echo "  BUILD FAILED"; fail=1; continue; }

    # Symbol parity for THIS configuration's .so (Phase D).
    outdir="target/$([[ $profile == --release ]] && echo release || echo debug)"
    R_SO="$outdir/libdriver.so"
    if [[ ! -f $R_SO ]]; then echo "  missing $R_SO"; fail=1; continue; fi

    c_syms=$(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u)
    r_syms=$(nm -D --defined-only "$R_SO"  | awk '{print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [[ -n $missing ]]; then
      echo "  SYMBOL PARITY FAILED — C exports missing from Rust:"
      echo "$missing" | sed 's/^/    /'
      fail=1
    else
      echo "  symbol parity: OK (C exports: $(echo "$c_syms" | wc -l), all present in Rust)"
    fi

    # Undefined non-libc symbols in the Rust .so.
    undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' \
      | grep -vE '^(_ITM_|_Unwind_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|statx|gettid)' \
      | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev)$' \
      | sort -u)
    if [[ -n $undef ]]; then
      echo "  UNRESOLVED non-libc symbols:"; echo "$undef" | sed 's/^/    /'; fail=1
    else
      echo "  undefined non-libc symbols: 0"
    fi

    # shellcheck disable=SC2086
    timeout 600 cargo test $profile $combo --no-fail-fast > /tmp/tb_test.log 2>&1
    rc=$?
    grep -E '^test result:' /tmp/tb_test.log | sed 's/^/    /'
    if (( rc != 0 )) || grep -qE '^test result: FAILED' /tmp/tb_test.log; then
      echo "  TESTS FAILED (cargo exit $rc)"
      grep -E '^test .* FAILED|^ *[a-z_0-9]+$' /tmp/tb_test.log | head -30 | sed 's/^/    /'
      fail=1
    fi
    # Guard against a run that silently executed nothing.
    if ! grep -qE '^test result: ok\. [1-9]' /tmp/tb_test.log; then
      echo "  NO TESTS RAN — treating as failure"; fail=1
    fi
  done
done

note "RESULT"
if (( fail )); then echo "FAILURES PRESENT"; exit 1; else echo "ALL CONFIGURATIONS PASS"; fi
