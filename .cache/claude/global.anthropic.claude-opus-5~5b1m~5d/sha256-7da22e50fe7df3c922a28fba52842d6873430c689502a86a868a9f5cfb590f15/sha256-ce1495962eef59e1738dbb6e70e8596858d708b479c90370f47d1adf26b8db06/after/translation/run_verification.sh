#!/usr/bin/env bash
# Full differential verification run.
#
#   ./run_verification.sh
#
# 1. builds the C shared library (ground truth),
# 2. enumerates every feature combination declared in Cargo.toml,
# 3. for each combination x {debug, release} builds the Rust cdylib and runs the
#    differential test suite against it,
# 4. diffs `nm -D` symbol sets.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
C_DIR="$CRATE_DIR/../c_src"
CARGO="cargo --offline"
fail=0

echo "== building C ground truth =="
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$C_DIR/build/libdriver.so"

# --- feature combinations -----------------------------------------------------
# Extracted mechanically from Cargo.toml's [features] section (powerset).
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "default" && $0 != "") print}' Cargo.toml
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "== no [features] in Cargo.toml: the only configuration is the default =="
  COMBOS=("__default__")
else
  COMBOS=("__default__" "__none__")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) featflags=() ; label="default features" ;;
    __none__)    featflags=(--no-default-features) ; label="no default features" ;;
    *)           featflags=(--no-default-features --features "$combo")
                 label="features: $combo" ;;
  esac

  for profile in debug release; do
    relflag=()
    [ "$profile" = release ] && relflag=(--release)
    echo
    echo "== [$label | $profile] build cdylib + run differential tests =="

    if ! $CARGO build "${relflag[@]}" "${featflags[@]}" >/dev/null 2>&1; then
      echo "  BUILD FAILED"; fail=1; continue
    fi
    RUST_SO="$CRATE_DIR/target/$profile/libdriver.so"
    if [ ! -f "$RUST_SO" ]; then echo "  cdylib missing: $RUST_SO"; fail=1; continue; fi

    # Symbol parity for this configuration.
    c_syms=$(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort)
    r_syms=$(nm -D --defined-only "$RUST_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      echo "  SYMBOL PARITY FAIL - missing from Rust .so:"; echo "$missing" | sed 's/^/    /'
      fail=1
    else
      echo "  symbol parity: OK ($(echo "$c_syms" | wc -l) C symbol(s), 0 missing)"
    fi

    # Undefined non-libc symbols in the Rust .so.
    undef=$(nm -D -u "$RUST_SO" | awk '{print $2}' | grep -v '^__\?' \
            | grep -vE '^(printf|setlocale|memcpy|memset|memcmp|abort|free|malloc|realloc|calloc|strlen|write|dl_iterate_phdr|pthread_.*|_Unwind_.*|getenv|sysconf|open|close|read|mmap|munmap|posix_memalign|bcmp|syscall|dlsym|dladdr|gnu_get_libc_version|.*@.*)$' || true)
    if [ -n "$undef" ]; then
      echo "  NOTE unresolved non-libc imports:"; echo "$undef" | sed 's/^/    /'
    fi

    if DRIVER_RUST_SO="$RUST_SO" timeout 600 $CARGO test "${relflag[@]}" \
         "${featflags[@]}" -- --test-threads=1 > "$CRATE_DIR/target/test-$profile.log" 2>&1; then
      grep -E '^test result:' "$CRATE_DIR/target/test-$profile.log" | sed 's/^/  /'
    else
      echo "  TESTS FAILED (see target/test-$profile.log)"
      tail -n 30 "$CRATE_DIR/target/test-$profile.log" | sed 's/^/    /'
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
